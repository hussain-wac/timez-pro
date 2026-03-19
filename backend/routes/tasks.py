from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func
from datetime import datetime, timezone, timedelta
import logging

from database import get_db
from models import Task, TimeEntry, User, TaskAssignment, Project
from schemas import (
    TaskCreate,
    TaskResponse,
    TaskWithTotalTime,
    TaskWithAssignees,
    TaskAssigneeInfo,
    TimeEntryResponse,
    SyncTimeRequest,
    TaskUpdate,
    ProjectWithTasks,
    TaskInProjectGroup,
    UserInfo,
)
from auth import get_current_user

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/tasks", tags=["tasks"])


def get_user_info(user: User) -> UserInfo:
    """Convert User model to UserInfo schema"""
    return UserInfo(
        id=user.id,
        email=user.email,
        name=user.name,
        picture=user.picture,
    )


def is_user_assigned_to_task(db: Session, task_id: int, user_id: int) -> bool:
    """Check if a user is assigned to a task."""
    assignment = (
        db.query(TaskAssignment)
        .filter(TaskAssignment.task_id == task_id, TaskAssignment.user_id == user_id)
        .first()
    )
    return assignment is not None


@router.get("", response_model=list[TaskWithAssignees])
def list_tasks(
    status: str = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List all tasks assigned to the current user with their total tracked time."""
    # Get task IDs assigned to user
    assignments = db.query(TaskAssignment).filter(TaskAssignment.user_id == current_user.id).all()
    task_ids = [a.task_id for a in assignments]

    query = db.query(Task).filter(Task.id.in_(task_ids))
    if status:
        query = query.filter(Task.status == status)
    tasks = query.all()

    result = []
    for task in tasks:
        project = db.query(Project).filter(Project.id == task.project_id).first()

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        max_seconds = task.max_hours * 3600
        remaining_seconds = max(0, max_seconds - total_seconds)

        # Get all assignees
        all_assignments = db.query(TaskAssignment).filter(TaskAssignment.task_id == task.id).all()
        assignees = []
        for a in all_assignments:
            user = db.query(User).filter(User.id == a.user_id).first()
            assignees.append(TaskAssigneeInfo(
                id=a.id,
                user_id=a.user_id,
                user=get_user_info(user),
                is_primary=a.is_primary,
                assigned_at=a.assigned_at,
            ))

        result.append(TaskWithAssignees(
            id=task.id,
            project_id=task.project_id,
            name=task.name,
            description=task.description,
            max_hours=task.max_hours,
            status=task.status,
            priority=task.priority,
            due_date=task.due_date,
            created_by=task.created_by,
            created_at=task.created_at,
            updated_at=task.updated_at,
            total_tracked_seconds=total_seconds,
            remaining_seconds=remaining_seconds,
            assignees=assignees,
            project_name=project.name if project else None,
            project_color=project.color if project else None,
        ))

    return result


@router.get("/timer", response_model=list[ProjectWithTasks])
def list_timer_tasks(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    List tasks for timer - only in_progress status with total tracked time.
    Returns tasks grouped by project.
    """
    # Get task IDs assigned to user with in_progress status
    assignments = db.query(TaskAssignment).filter(TaskAssignment.user_id == current_user.id).all()
    task_ids = [a.task_id for a in assignments]
    assignment_lookup = {a.task_id: a for a in assignments}

    tasks = db.query(Task).filter(
        Task.id.in_(task_ids),
        Task.status == "in_progress"
    ).all()

    # Group tasks by project
    project_tasks = {}
    for task in tasks:
        if task.project_id not in project_tasks:
            project_tasks[task.project_id] = []

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        max_seconds = task.max_hours * 3600
        remaining_seconds = max(0, max_seconds - total_seconds)

        assignment = assignment_lookup.get(task.id)
        is_primary_assignee = assignment.is_primary if assignment else False

        project_tasks[task.project_id].append(TaskInProjectGroup(
            id=task.id,
            name=task.name,
            description=task.description,
            max_hours=task.max_hours,
            status=task.status,
            priority=task.priority,
            due_date=task.due_date,
            total_tracked_seconds=total_seconds,
            remaining_seconds=remaining_seconds,
            is_primary_assignee=is_primary_assignee,
        ))

    # Build response
    result = []
    for project_id, tasks_list in project_tasks.items():
        project = db.query(Project).filter(Project.id == project_id).first()
        if project:
            result.append(ProjectWithTasks(
                id=project.id,
                name=project.name,
                description=project.description,
                status=project.status,
                color=project.color,
                tasks=tasks_list,
            ))

    return result


@router.get("/{task_id}", response_model=TaskWithAssignees)
def get_task(
    task_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get a single task with its total tracked time."""
    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    # Check access: user must be assigned or admin
    if not current_user.is_admin and not is_user_assigned_to_task(db, task_id, current_user.id):
        raise HTTPException(status_code=403, detail="Not assigned to this task")

    project = db.query(Project).filter(Project.id == task.project_id).first()

    total_seconds = (
        db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
        .filter(TimeEntry.task_id == task_id, TimeEntry.duration.isnot(None))
        .scalar()
    ) or 0

    max_seconds = task.max_hours * 3600
    remaining_seconds = max(0, max_seconds - total_seconds)

    # Get all assignees
    all_assignments = db.query(TaskAssignment).filter(TaskAssignment.task_id == task.id).all()
    assignees = []
    for a in all_assignments:
        user = db.query(User).filter(User.id == a.user_id).first()
        assignees.append(TaskAssigneeInfo(
            id=a.id,
            user_id=a.user_id,
            user=get_user_info(user),
            is_primary=a.is_primary,
            assigned_at=a.assigned_at,
        ))

    return TaskWithAssignees(
        id=task.id,
        project_id=task.project_id,
        name=task.name,
        description=task.description,
        max_hours=task.max_hours,
        status=task.status,
        priority=task.priority,
        due_date=task.due_date,
        created_by=task.created_by,
        created_at=task.created_at,
        updated_at=task.updated_at,
        total_tracked_seconds=total_seconds,
        remaining_seconds=remaining_seconds,
        assignees=assignees,
        project_name=project.name if project else None,
        project_color=project.color if project else None,
    )


@router.put("/{task_id}", response_model=TaskResponse)
def update_task(
    task_id: int,
    task_data: TaskUpdate,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Update a task. Admin only."""
    if not current_user.is_admin:
        raise HTTPException(status_code=403, detail="Admin access required")

    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    if task_data.name is not None:
        task.name = task_data.name
    if task_data.description is not None:
        task.description = task_data.description
    if task_data.max_hours is not None:
        task.max_hours = task_data.max_hours
    if task_data.priority is not None:
        task.priority = task_data.priority
    if task_data.due_date is not None:
        task.due_date = task_data.due_date
    if task_data.status is not None:
        task.status = task_data.status

    db.commit()
    db.refresh(task)
    return task


@router.delete("/{task_id}", status_code=204)
def delete_task(
    task_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Delete a task and all its time entries. Admin only."""
    if not current_user.is_admin:
        raise HTTPException(status_code=403, detail="Admin access required")

    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    db.delete(task)
    db.commit()
    return None


@router.get("/{task_id}/entries", response_model=list[TimeEntryResponse])
def list_task_entries(
    task_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List all time entries for a task."""
    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    # Check access: user must be assigned or admin
    if not current_user.is_admin and not is_user_assigned_to_task(db, task_id, current_user.id):
        raise HTTPException(status_code=403, detail="Not assigned to this task")

    return (
        db.query(TimeEntry)
        .filter(TimeEntry.task_id == task_id)
        .order_by(TimeEntry.start_time.desc())
        .all()
    )


@router.post("/sync-time", response_model=TimeEntryResponse)
def sync_time(
    request: SyncTimeRequest,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Sync time slot from desktop app.

    The desktop app sends incremental time slots (seconds tracked since last sync).
    Backend ADDS the slot_seconds to the running entry's duration.
    Time can only increment - never decrement.

    Request:
    - task_id: Task being tracked
    - slot_seconds: Seconds tracked in THIS slot (since last sync)
    - session_start: When the timer session started
    - slot_end: Current time or stop time
    - is_final: True if timer was stopped
    """
    logger.info(f"[sync-time] task_id={request.task_id}, slot_seconds={request.slot_seconds}, is_final={request.is_final}, user={current_user.id}")

    # Validate slot_seconds is positive
    if request.slot_seconds < 0:
        logger.warning(f"[sync-time] Rejected negative slot_seconds: {request.slot_seconds}")
        raise HTTPException(status_code=400, detail="slot_seconds cannot be negative")

    # Skip if slot is 0 seconds (no time to add)
    if request.slot_seconds == 0 and not request.is_final:
        logger.info(f"[sync-time] Skipping 0-second slot for task {request.task_id}")
        # Return existing entry or create placeholder
        running = (
            db.query(TimeEntry)
            .filter(
                TimeEntry.task_id == request.task_id,
                TimeEntry.user_id == current_user.id,
                TimeEntry.end_time.is_(None),
            )
            .first()
        )
        if running:
            return running

    task = db.query(Task).filter(Task.id == request.task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    # Check if user is assigned to this task
    if not is_user_assigned_to_task(db, request.task_id, current_user.id):
        # Auto-assign if user is a project member
        from models import ProjectMember
        is_project_member = (
            db.query(ProjectMember)
            .filter(ProjectMember.project_id == task.project_id, ProjectMember.user_id == current_user.id)
            .first()
        )
        if not is_project_member:
            raise HTTPException(status_code=403, detail="Not a member of this project")

        # Auto-assign user to the task
        logger.info(f"[sync-time] Auto-assigning user {current_user.id} to task {request.task_id}")
        assignment = TaskAssignment(
            task_id=request.task_id,
            user_id=current_user.id,
            assigned_by=current_user.id,
            is_primary=True,
        )
        db.add(assignment)
        db.commit()

    # Find running (open) entry for this task and user
    running = (
        db.query(TimeEntry)
        .filter(
            TimeEntry.task_id == request.task_id,
            TimeEntry.user_id == current_user.id,
            TimeEntry.end_time.is_(None),
        )
        .first()
    )

    if running:
        # ADD the slot_seconds to existing duration (never subtract)
        old_duration = running.duration or 0
        new_duration = old_duration + request.slot_seconds

        logger.info(f"[sync-time] Adding to entry {running.id}: {old_duration} + {request.slot_seconds} = {new_duration}")

        running.duration = new_duration
        running.client_stopped_at = request.slot_end

        if request.is_final:
            running.end_time = request.slot_end
            running.is_synced = True
            logger.info(f"[sync-time] Session ended. Entry {running.id} finalized with duration={new_duration}")

        db.commit()
        db.refresh(running)
    else:
        # Create new entry for this session
        logger.info(f"[sync-time] Creating new entry for task {request.task_id}, initial duration={request.slot_seconds}")
        entry = TimeEntry(
            task_id=request.task_id,
            user_id=current_user.id,
            project_id=task.project_id,
            start_time=request.session_start,
            end_time=request.slot_end if request.is_final else None,
            duration=request.slot_seconds,
            client_started_at=request.session_start,
            client_stopped_at=request.slot_end,
            is_synced=request.is_final,
        )
        db.add(entry)
        db.commit()
        db.refresh(entry)
        running = entry

    # Debug: check total for this task
    total = (
        db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
        .filter(TimeEntry.task_id == request.task_id, TimeEntry.duration.isnot(None))
        .scalar()
    ) or 0
    logger.info(f"[sync-time] Task {request.task_id} total duration: {total} seconds")

    # Update user's current task
    if request.is_final:
        current_user.current_task_id = None
    else:
        if task.status == "todo":
            task.status = "in_progress"
        current_user.current_task_id = request.task_id

    db.commit()
    db.refresh(task)

    return running
