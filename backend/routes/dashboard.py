from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func
from datetime import datetime, timezone, timedelta
from typing import Optional
from calendar import monthrange

from database import get_db
from models import Task, TimeEntry, User, Project, ProjectMember, TaskAssignment
from schemas import (
    UserResponse,
    TaskStatusUpdate,
    EmployeeWithHours,
    TaskCreate,
    UserDailySummary,
    TaskDailySummary,
    UserInfo,
)
from auth import get_current_user

router = APIRouter(prefix="/api/dashboard", tags=["dashboard"])


@router.get("/me")
def get_current_user_info(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get current user info."""
    return {
        "id": current_user.id,
        "email": current_user.email,
        "name": current_user.name,
        "picture": current_user.picture,
        "is_admin": current_user.is_admin,
    }


@router.get("/users")
def get_all_users(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get all registered users (employees)."""
    if not current_user.is_admin:
        return []
    users = db.query(User).all()
    return [
        {
            "id": u.id,
            "email": u.email,
            "name": u.name,
            "picture": u.picture,
            "is_admin": u.is_admin,
            "created_at": u.created_at.isoformat() if u.created_at else None,
        }
        for u in users
    ]


@router.get("/users/{user_id}/status")
def get_user_timer_status(
    user_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get current timer status for a specific user."""
    running = (
        db.query(TimeEntry)
        .filter(
            TimeEntry.user_id == user_id,
            TimeEntry.end_time.is_(None),
        )
        .first()
    )

    if not running:
        return {
            "running": False,
            "user_id": user_id,
        }

    task = db.query(Task).filter(Task.id == running.task_id).first()
    project = db.query(Project).filter(Project.id == task.project_id).first() if task else None
    now = datetime.now(timezone.utc)
    elapsed = int((now - running.start_time.replace(tzinfo=timezone.utc)).total_seconds())

    return {
        "running": True,
        "user_id": user_id,
        "task_id": running.task_id,
        "task_name": task.name if task else None,
        "project_id": project.id if project else None,
        "project_name": project.name if project else None,
        "start_time": running.start_time.isoformat() if running.start_time else None,
        "elapsed_seconds": elapsed,
    }


@router.get("/users-status")
def get_all_users_status(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get timer status for all users (real-time)."""
    if not current_user.is_admin:
        return []
    users = db.query(User).all()
    now = datetime.now(timezone.utc)

    result = []
    for user in users:
        running = (
            db.query(TimeEntry)
            .filter(
                TimeEntry.user_id == user.id,
                TimeEntry.end_time.is_(None),
            )
            .first()
        )

        if running:
            task = db.query(Task).filter(Task.id == running.task_id).first()
            project = db.query(Project).filter(Project.id == task.project_id).first() if task else None
            elapsed = int((now - running.start_time.replace(tzinfo=timezone.utc)).total_seconds())
            result.append({
                "user_id": user.id,
                "user_name": user.name or user.email,
                "user_picture": user.picture,
                "running": True,
                "task_id": running.task_id,
                "task_name": task.name if task else None,
                "project_id": project.id if project else None,
                "project_name": project.name if project else None,
                "start_time": running.start_time.isoformat() if running.start_time else None,
                "elapsed_seconds": elapsed,
            })
        else:
            result.append({
                "user_id": user.id,
                "user_name": user.name or user.email,
                "user_picture": user.picture,
                "running": False,
            })

    return result


@router.get("/stats")
def get_dashboard_stats(
    project_id: int = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get dashboard statistics. Optionally filter by project."""
    if not current_user.is_admin:
        return {
            "total_users": 0,
            "total_projects": 0,
            "total_tasks": 0,
            "currently_working": 0,
            "today_total_seconds": 0,
        }

    total_users = db.query(User).count()
    total_projects = db.query(Project).filter(Project.status == "active").count()

    if project_id:
        total_tasks = db.query(Task).filter(Task.project_id == project_id).count()
        running_entries = (
            db.query(TimeEntry)
            .filter(TimeEntry.end_time.is_(None), TimeEntry.project_id == project_id)
            .count()
        )
    else:
        total_tasks = db.query(Task).count()
        running_entries = (
            db.query(TimeEntry)
            .filter(TimeEntry.end_time.is_(None))
            .count()
        )

    today = datetime.now(timezone.utc).date()
    today_start = datetime.combine(today, datetime.min.time()).replace(tzinfo=timezone.utc)

    today_query = db.query(func.coalesce(func.sum(TimeEntry.duration), 0)).filter(
        TimeEntry.start_time >= today_start,
        TimeEntry.duration.isnot(None),
    )
    if project_id:
        today_query = today_query.filter(TimeEntry.project_id == project_id)

    today_total = today_query.scalar() or 0

    return {
        "total_users": total_users,
        "total_projects": total_projects,
        "total_tasks": total_tasks,
        "currently_working": running_entries,
        "today_total_seconds": today_total,
    }


@router.get("/tasks")
def get_all_tasks_with_time(
    project_id: int = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get all tasks with time tracked. Optionally filter by project."""
    if not current_user.is_admin:
        return []

    query = db.query(Task)
    if project_id:
        query = query.filter(Task.project_id == project_id)
    tasks = query.all()

    result = []
    for task in tasks:
        project = db.query(Project).filter(Project.id == task.project_id).first()

        # Get assignees
        assignments = db.query(TaskAssignment).filter(TaskAssignment.task_id == task.id).all()
        assignees = []
        for a in assignments:
            user = db.query(User).filter(User.id == a.user_id).first()
            if user:
                assignees.append({
                    "id": user.id,
                    "name": user.name,
                    "email": user.email,
                    "picture": user.picture,
                    "is_primary": a.is_primary,
                })

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        max_seconds = task.max_hours * 3600
        remaining = max(0, max_seconds - total_seconds)

        running = (
            db.query(TimeEntry)
            .filter(
                TimeEntry.task_id == task.id,
                TimeEntry.end_time.is_(None),
            )
            .first()
        )

        result.append({
            "id": task.id,
            "name": task.name,
            "description": task.description,
            "max_hours": task.max_hours,
            "priority": task.priority,
            "due_date": task.due_date.isoformat() if task.due_date else None,
            "total_tracked_seconds": total_seconds,
            "remaining_seconds": remaining,
            "project_id": task.project_id,
            "project_name": project.name if project else None,
            "project_color": project.color if project else None,
            "assignees": assignees,
            "is_running": running is not None,
            "status": task.status,
            "created_at": task.created_at.isoformat() if task.created_at else None,
        })

    return result


@router.get("/employees")
def get_employees_with_hours(
    year: int = None,
    month: int = None,
    project_id: int = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get all employees with their monthly working hours. Optionally filter by project."""
    if not current_user.is_admin:
        return []

    now = datetime.now(timezone.utc)
    year = year or now.year
    month = month or now.month

    _, last_day = monthrange(year, month)
    month_start = datetime(year, month, 1, tzinfo=timezone.utc)
    month_end = datetime(year, month, last_day, 23, 59, 59, tzinfo=timezone.utc)

    if project_id:
        # Filter users who are members of this project
        member_ids = [
            m.user_id for m in
            db.query(ProjectMember).filter(ProjectMember.project_id == project_id).all()
        ]
        users = db.query(User).filter(User.id.in_(member_ids)).all()
    else:
        users = db.query(User).all()

    result = []

    for user in users:
        time_query = db.query(func.coalesce(func.sum(TimeEntry.duration), 0)).filter(
            TimeEntry.user_id == user.id,
            TimeEntry.start_time >= month_start,
            TimeEntry.start_time <= month_end,
            TimeEntry.duration.isnot(None),
        )
        if project_id:
            time_query = time_query.filter(TimeEntry.project_id == project_id)

        total_seconds = time_query.scalar() or 0

        # Count tasks assigned to user
        task_count_query = (
            db.query(TaskAssignment)
            .join(Task, TaskAssignment.task_id == Task.id)
            .filter(TaskAssignment.user_id == user.id)
        )
        if project_id:
            task_count_query = task_count_query.filter(Task.project_id == project_id)

        task_count = task_count_query.count()

        # Count projects user is a member of
        project_count = db.query(ProjectMember).filter(ProjectMember.user_id == user.id).count()

        result.append({
            "id": user.id,
            "email": user.email,
            "name": user.name,
            "picture": user.picture,
            "monthly_hours": round(total_seconds / 3600, 1),
            "task_count": task_count,
            "project_count": project_count,
        })

    return result


@router.get("/kanban/{user_id}")
def get_kanban_tasks(
    user_id: int,
    project_id: int = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get tasks in kanban format for a specific user. Optionally filter by project."""
    if not current_user.is_admin:
        return []

    # Get tasks assigned to the user
    task_ids_query = db.query(TaskAssignment.task_id).filter(TaskAssignment.user_id == user_id)
    task_ids = [t[0] for t in task_ids_query.all()]

    query = db.query(Task).filter(Task.id.in_(task_ids))
    if project_id:
        query = query.filter(Task.project_id == project_id)

    tasks = query.all()

    kanban = {
        "todo": [],
        "in_progress": [],
        "review": [],
        "done": [],
    }

    for task in tasks:
        project = db.query(Project).filter(Project.id == task.project_id).first()

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        task_data = {
            "id": task.id,
            "name": task.name,
            "description": task.description,
            "max_hours": task.max_hours,
            "priority": task.priority,
            "total_tracked_seconds": total_seconds,
            "status": task.status,
            "project_id": task.project_id,
            "project_name": project.name if project else None,
            "project_color": project.color if project else None,
            "created_at": task.created_at.isoformat() if task.created_at else None,
        }

        status = task.status or "todo"
        if status in kanban:
            kanban[status].append(task_data)

    return kanban


@router.post("/tasks/{task_id}/status")
def update_task_status(
    task_id: int,
    request: TaskStatusUpdate,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Update task status for kanban."""
    if not current_user.is_admin:
        raise HTTPException(status_code=403, detail="Not authorized")

    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    old_status = task.status

    # If moving from in_progress to another status, clear current_task_id for all assigned users
    if old_status == "in_progress" and request.status != "in_progress":
        assignments = db.query(TaskAssignment).filter(TaskAssignment.task_id == task_id).all()
        for a in assignments:
            user = db.query(User).filter(User.id == a.user_id).first()
            if user and user.current_task_id == task_id:
                user.current_task_id = None

    task.status = request.status
    db.commit()
    db.refresh(task)

    return {"id": task.id, "status": task.status}


@router.post("/tasks")
def create_task_admin(
    request: TaskCreate,
    project_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Create a new task in a project (admin)."""
    if not current_user.is_admin:
        raise HTTPException(status_code=403, detail="Not authorized")

    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    task = Task(
        project_id=project_id,
        name=request.name,
        description=request.description,
        max_hours=request.max_hours,
        priority=request.priority,
        due_date=request.due_date,
        status=request.status,
        created_by=current_user.id,
    )
    db.add(task)
    db.commit()
    db.refresh(task)

    return {
        "id": task.id,
        "name": task.name,
        "description": task.description,
        "max_hours": task.max_hours,
        "priority": task.priority,
        "project_id": task.project_id,
        "project_name": project.name,
        "status": task.status,
    }


@router.get("/user-daily-summary", response_model=UserDailySummary)
def get_user_daily_summary(
    date: Optional[str] = None,
    user_id: Optional[int] = None,
    project_id: Optional[int] = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Get daily time tracking summary for a user.

    - If no date is provided, returns today's summary
    - If no user_id is provided, returns current user's summary
    - Admins can query any user, non-admins can only query themselves
    - Can optionally filter by project

    Query parameters:
    - date: YYYY-MM-DD format (optional, defaults to today)
    - user_id: User ID to query (optional, defaults to current user)
    - project_id: Project ID to filter by (optional)

    Returns:
    - Date
    - List of tasks worked on that day with time per task
    - Total work time for the day
    - User info
    """
    # Determine target user
    target_user_id = user_id if user_id else current_user.id

    # Check authorization: non-admins can only view their own data
    if not current_user.is_admin and target_user_id != current_user.id:
        raise HTTPException(
            status_code=403,
            detail="Not authorized to view other users' data"
        )

    # Get target user
    target_user = db.query(User).filter(User.id == target_user_id).first()
    if not target_user:
        raise HTTPException(status_code=404, detail="User not found")

    # Parse date or use today
    if date:
        try:
            query_date = datetime.strptime(date, "%Y-%m-%d").date()
        except ValueError:
            raise HTTPException(
                status_code=400,
                detail="Invalid date format. Use YYYY-MM-DD"
            )
    else:
        query_date = datetime.now(timezone.utc).date()

    # Calculate date range for the query
    day_start = datetime.combine(query_date, datetime.min.time()).replace(tzinfo=timezone.utc)
    day_end = datetime.combine(query_date, datetime.max.time()).replace(tzinfo=timezone.utc)

    # Query time entries for the user on the specified date
    time_entries_query = (
        db.query(TimeEntry)
        .filter(
            TimeEntry.user_id == target_user_id,
            TimeEntry.start_time >= day_start,
            TimeEntry.start_time <= day_end,
            TimeEntry.duration.isnot(None),  # Only completed entries
        )
    )
    if project_id:
        time_entries_query = time_entries_query.filter(TimeEntry.project_id == project_id)

    time_entries = time_entries_query.all()

    # Aggregate time by task
    task_time_map = {}  # task_id -> {total_seconds, entries_count}

    for entry in time_entries:
        task_id = entry.task_id
        if task_id not in task_time_map:
            task_time_map[task_id] = {
                "total_seconds": 0,
                "entries_count": 0,
            }
        task_time_map[task_id]["total_seconds"] += entry.duration or 0
        task_time_map[task_id]["entries_count"] += 1

    # Build task summaries
    task_summaries = []
    for task_id, time_data in task_time_map.items():
        task = db.query(Task).filter(Task.id == task_id).first()
        if task:
            task_summaries.append(
                TaskDailySummary(
                    task_id=task.id,
                    task_name=task.name,
                    status=task.status or "todo",
                    total_seconds=time_data["total_seconds"],
                    time_entries_count=time_data["entries_count"],
                )
            )

    # Sort tasks by time spent (descending)
    task_summaries.sort(key=lambda x: x.total_seconds, reverse=True)

    # Calculate totals
    total_work_seconds = sum(t.total_seconds for t in task_summaries)

    # Build user info
    user_info = UserInfo(
        id=target_user.id,
        email=target_user.email,
        name=target_user.name,
        picture=target_user.picture,
    )

    return UserDailySummary(
        date=query_date.isoformat(),
        user=user_info,
        tasks=task_summaries,
        total_work_seconds=total_work_seconds,
        total_tasks_worked=len(task_summaries),
    )


# ============================================================================
# Project-related dashboard endpoints
# ============================================================================

@router.get("/projects/summary")
def get_projects_summary(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get summary of all projects with stats."""
    if not current_user.is_admin:
        return []

    projects = db.query(Project).filter(Project.status == "active").all()

    result = []
    for project in projects:
        member_count = db.query(ProjectMember).filter(ProjectMember.project_id == project.id).count()
        task_count = db.query(Task).filter(Task.project_id == project.id).count()

        # Tasks by status
        tasks_by_status = {}
        for status in ["todo", "in_progress", "review", "done"]:
            count = db.query(Task).filter(
                Task.project_id == project.id,
                Task.status == status
            ).count()
            tasks_by_status[status] = count

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.project_id == project.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        # Currently working on this project
        currently_working = (
            db.query(TimeEntry)
            .filter(TimeEntry.project_id == project.id, TimeEntry.end_time.is_(None))
            .count()
        )

        result.append({
            "id": project.id,
            "name": project.name,
            "description": project.description,
            "status": project.status,
            "color": project.color,
            "member_count": member_count,
            "task_count": task_count,
            "tasks_by_status": tasks_by_status,
            "total_tracked_seconds": total_seconds,
            "currently_working": currently_working,
            "created_at": project.created_at.isoformat() if project.created_at else None,
        })

    return result
