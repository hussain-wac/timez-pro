from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func
from typing import List

from database import get_db
from models import Project, ProjectMember, Task, TaskAssignment, TimeEntry, User
from schemas import (
    ProjectCreate,
    ProjectUpdate,
    ProjectResponse,
    ProjectWithDetails,
    ProjectWithMembers,
    ProjectMemberInfo,
    AddProjectMembersRequest,
    TaskCreate,
    TaskResponse,
    TaskWithTotalTime,
    TaskWithAssignees,
    TaskAssigneeInfo,
    AssignTaskRequest,
    UserInfo,
    ProjectWithTasks,
    TaskInProjectGroup,
)
from auth import get_current_user

router = APIRouter(prefix="/api", tags=["projects"])


def require_admin(user: User):
    """Helper to check admin status"""
    if not user.is_admin:
        raise HTTPException(status_code=403, detail="Admin access required")


def get_user_info(user: User) -> UserInfo:
    """Convert User model to UserInfo schema"""
    return UserInfo(
        id=user.id,
        email=user.email,
        name=user.name,
        picture=user.picture,
    )


# ============================================================================
# Project Management Endpoints (Admin only)
# ============================================================================

@router.get("/projects", response_model=List[ProjectWithDetails])
def list_projects(
    status: str = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List all projects. Admin sees all, users see their allocated projects."""
    if current_user.is_admin:
        query = db.query(Project)
        if status:
            query = query.filter(Project.status == status)
        projects = query.all()
    else:
        # Non-admin users see only their allocated projects
        query = (
            db.query(Project)
            .join(ProjectMember, Project.id == ProjectMember.project_id)
            .filter(ProjectMember.user_id == current_user.id)
        )
        if status:
            query = query.filter(Project.status == status)
        projects = query.all()

    result = []
    for project in projects:
        creator = db.query(User).filter(User.id == project.created_by).first()
        member_count = db.query(ProjectMember).filter(ProjectMember.project_id == project.id).count()
        task_count = db.query(Task).filter(Task.project_id == project.id).count()

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.project_id == project.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        result.append(ProjectWithDetails(
            id=project.id,
            name=project.name,
            description=project.description,
            status=project.status,
            color=project.color,
            created_by=project.created_by,
            created_at=project.created_at,
            updated_at=project.updated_at,
            creator=get_user_info(creator) if creator else None,
            member_count=member_count,
            task_count=task_count,
            total_tracked_seconds=total_seconds,
        ))

    return result


@router.post("/projects", response_model=ProjectResponse, status_code=201)
def create_project(
    project_data: ProjectCreate,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Create a new project. Admin only."""
    require_admin(current_user)

    project = Project(
        name=project_data.name,
        description=project_data.description,
        color=project_data.color,
        created_by=current_user.id,
    )
    db.add(project)
    db.commit()
    db.refresh(project)
    return project


@router.get("/projects/{project_id}", response_model=ProjectWithDetails)
def get_project(
    project_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get project details."""
    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    # Check access: admin or member
    if not current_user.is_admin:
        member = (
            db.query(ProjectMember)
            .filter(ProjectMember.project_id == project_id, ProjectMember.user_id == current_user.id)
            .first()
        )
        if not member:
            raise HTTPException(status_code=403, detail="Not a member of this project")

    creator = db.query(User).filter(User.id == project.created_by).first()
    member_count = db.query(ProjectMember).filter(ProjectMember.project_id == project.id).count()
    task_count = db.query(Task).filter(Task.project_id == project.id).count()

    total_seconds = (
        db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
        .filter(TimeEntry.project_id == project.id, TimeEntry.duration.isnot(None))
        .scalar()
    ) or 0

    return ProjectWithDetails(
        id=project.id,
        name=project.name,
        description=project.description,
        status=project.status,
        color=project.color,
        created_by=project.created_by,
        created_at=project.created_at,
        updated_at=project.updated_at,
        creator=get_user_info(creator) if creator else None,
        member_count=member_count,
        task_count=task_count,
        total_tracked_seconds=total_seconds,
    )


@router.put("/projects/{project_id}", response_model=ProjectResponse)
def update_project(
    project_id: int,
    project_data: ProjectUpdate,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Update a project. Admin only."""
    require_admin(current_user)

    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    if project_data.name is not None:
        project.name = project_data.name
    if project_data.description is not None:
        project.description = project_data.description
    if project_data.status is not None:
        project.status = project_data.status
    if project_data.color is not None:
        project.color = project_data.color

    db.commit()
    db.refresh(project)
    return project


@router.delete("/projects/{project_id}", status_code=204)
def delete_project(
    project_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Archive/delete a project. Admin only."""
    require_admin(current_user)

    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    # Soft delete by setting status to archived
    project.status = "archived"
    db.commit()
    return None


# ============================================================================
# Project Members Endpoints (Admin only)
# ============================================================================

@router.get("/projects/{project_id}/members", response_model=List[ProjectMemberInfo])
def list_project_members(
    project_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List all members of a project."""
    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    # Check access: admin or member
    if not current_user.is_admin:
        member = (
            db.query(ProjectMember)
            .filter(ProjectMember.project_id == project_id, ProjectMember.user_id == current_user.id)
            .first()
        )
        if not member:
            raise HTTPException(status_code=403, detail="Not a member of this project")

    members = db.query(ProjectMember).filter(ProjectMember.project_id == project_id).all()

    result = []
    for m in members:
        user = db.query(User).filter(User.id == m.user_id).first()
        result.append(ProjectMemberInfo(
            id=m.id,
            user_id=m.user_id,
            user=get_user_info(user),
            role=m.role,
            allocated_at=m.allocated_at,
        ))

    return result


@router.post("/projects/{project_id}/members", response_model=List[ProjectMemberInfo])
def add_project_members(
    project_id: int,
    request: AddProjectMembersRequest,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Add members to a project. Admin only."""
    require_admin(current_user)

    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    added_members = []
    for user_id in request.user_ids:
        # Check if user exists
        user = db.query(User).filter(User.id == user_id).first()
        if not user:
            continue  # Skip non-existent users

        # Check if already a member
        existing = (
            db.query(ProjectMember)
            .filter(ProjectMember.project_id == project_id, ProjectMember.user_id == user_id)
            .first()
        )
        if existing:
            # Update role if different
            if existing.role != request.role:
                existing.role = request.role
                db.commit()
                db.refresh(existing)
            added_members.append(ProjectMemberInfo(
                id=existing.id,
                user_id=existing.user_id,
                user=get_user_info(user),
                role=existing.role,
                allocated_at=existing.allocated_at,
            ))
            continue

        # Add new member
        member = ProjectMember(
            project_id=project_id,
            user_id=user_id,
            role=request.role,
        )
        db.add(member)
        db.commit()
        db.refresh(member)

        added_members.append(ProjectMemberInfo(
            id=member.id,
            user_id=member.user_id,
            user=get_user_info(user),
            role=member.role,
            allocated_at=member.allocated_at,
        ))

    return added_members


@router.delete("/projects/{project_id}/members/{user_id}", status_code=204)
def remove_project_member(
    project_id: int,
    user_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Remove a member from a project. Admin only."""
    require_admin(current_user)

    member = (
        db.query(ProjectMember)
        .filter(ProjectMember.project_id == project_id, ProjectMember.user_id == user_id)
        .first()
    )
    if not member:
        raise HTTPException(status_code=404, detail="Member not found")

    # Also remove task assignments for this user in this project
    task_ids = [t.id for t in db.query(Task).filter(Task.project_id == project_id).all()]
    if task_ids:
        db.query(TaskAssignment).filter(
            TaskAssignment.task_id.in_(task_ids),
            TaskAssignment.user_id == user_id
        ).delete(synchronize_session=False)

    db.delete(member)
    db.commit()
    return None


# ============================================================================
# Tasks within Project Endpoints
# ============================================================================

@router.get("/projects/{project_id}/tasks", response_model=List[TaskWithAssignees])
def list_project_tasks(
    project_id: int,
    status: str = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List all tasks in a project."""
    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    # Check access: admin or member
    if not current_user.is_admin:
        member = (
            db.query(ProjectMember)
            .filter(ProjectMember.project_id == project_id, ProjectMember.user_id == current_user.id)
            .first()
        )
        if not member:
            raise HTTPException(status_code=403, detail="Not a member of this project")

    query = db.query(Task).filter(Task.project_id == project_id)
    if status:
        query = query.filter(Task.status == status)
    tasks = query.all()

    result = []
    for task in tasks:
        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        max_seconds = task.max_hours * 3600
        remaining_seconds = max(0, max_seconds - total_seconds)

        # Get assignees
        assignments = db.query(TaskAssignment).filter(TaskAssignment.task_id == task.id).all()
        assignees = []
        for a in assignments:
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
            project_name=project.name,
            project_color=project.color,
        ))

    return result


@router.post("/projects/{project_id}/tasks", response_model=TaskWithAssignees, status_code=201)
def create_project_task(
    project_id: int,
    task_data: TaskCreate,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Create a task in a project. Admin only."""
    require_admin(current_user)

    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        raise HTTPException(status_code=404, detail="Project not found")

    task = Task(
        project_id=project_id,
        name=task_data.name,
        description=task_data.description,
        max_hours=task_data.max_hours,
        priority=task_data.priority,
        due_date=task_data.due_date,
        status=task_data.status,
        created_by=current_user.id,
    )
    db.add(task)
    db.commit()
    db.refresh(task)

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
        total_tracked_seconds=0,
        remaining_seconds=task.max_hours * 3600,
        assignees=[],
        project_name=project.name,
        project_color=project.color,
    )


# ============================================================================
# Task Assignment Endpoints (Admin only)
# ============================================================================

@router.post("/tasks/{task_id}/assign", response_model=List[TaskAssigneeInfo])
def assign_task(
    task_id: int,
    request: AssignTaskRequest,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Assign users to a task. Admin only."""
    require_admin(current_user)

    task = db.query(Task).filter(Task.id == task_id).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    assigned = []
    for user_id in request.user_ids:
        # Check if user exists and is a project member
        user = db.query(User).filter(User.id == user_id).first()
        if not user:
            continue

        member = (
            db.query(ProjectMember)
            .filter(ProjectMember.project_id == task.project_id, ProjectMember.user_id == user_id)
            .first()
        )
        if not member:
            # Auto-add user to project if not a member
            member = ProjectMember(
                project_id=task.project_id,
                user_id=user_id,
                role="member",
            )
            db.add(member)

        # Check if already assigned
        existing = (
            db.query(TaskAssignment)
            .filter(TaskAssignment.task_id == task_id, TaskAssignment.user_id == user_id)
            .first()
        )
        if existing:
            # Update primary status
            is_primary = request.primary_user_id == user_id
            if existing.is_primary != is_primary:
                existing.is_primary = is_primary
                db.commit()
                db.refresh(existing)
            assigned.append(TaskAssigneeInfo(
                id=existing.id,
                user_id=existing.user_id,
                user=get_user_info(user),
                is_primary=existing.is_primary,
                assigned_at=existing.assigned_at,
            ))
            continue

        # Create new assignment
        assignment = TaskAssignment(
            task_id=task_id,
            user_id=user_id,
            assigned_by=current_user.id,
            is_primary=(request.primary_user_id == user_id),
        )
        db.add(assignment)
        db.commit()
        db.refresh(assignment)

        assigned.append(TaskAssigneeInfo(
            id=assignment.id,
            user_id=assignment.user_id,
            user=get_user_info(user),
            is_primary=assignment.is_primary,
            assigned_at=assignment.assigned_at,
        ))

    # If primary_user_id specified, ensure only that user is primary
    if request.primary_user_id:
        db.query(TaskAssignment).filter(
            TaskAssignment.task_id == task_id,
            TaskAssignment.user_id != request.primary_user_id
        ).update({"is_primary": False})
        db.commit()

    return assigned


@router.delete("/tasks/{task_id}/assign/{user_id}", status_code=204)
def unassign_task(
    task_id: int,
    user_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Unassign a user from a task. Admin only."""
    require_admin(current_user)

    assignment = (
        db.query(TaskAssignment)
        .filter(TaskAssignment.task_id == task_id, TaskAssignment.user_id == user_id)
        .first()
    )
    if not assignment:
        raise HTTPException(status_code=404, detail="Assignment not found")

    db.delete(assignment)
    db.commit()
    return None


# ============================================================================
# User-facing endpoints (My projects/tasks)
# ============================================================================

@router.get("/me/projects", response_model=List[ProjectWithDetails])
def get_my_projects(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get projects allocated to the current user."""
    memberships = (
        db.query(ProjectMember)
        .filter(ProjectMember.user_id == current_user.id)
        .all()
    )

    project_ids = [m.project_id for m in memberships]
    projects = db.query(Project).filter(
        Project.id.in_(project_ids),
        Project.status == "active"
    ).all()

    result = []
    for project in projects:
        creator = db.query(User).filter(User.id == project.created_by).first()
        member_count = db.query(ProjectMember).filter(ProjectMember.project_id == project.id).count()
        task_count = db.query(Task).filter(Task.project_id == project.id).count()

        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.project_id == project.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0

        result.append(ProjectWithDetails(
            id=project.id,
            name=project.name,
            description=project.description,
            status=project.status,
            color=project.color,
            created_by=project.created_by,
            created_at=project.created_at,
            updated_at=project.updated_at,
            creator=get_user_info(creator) if creator else None,
            member_count=member_count,
            task_count=task_count,
            total_tracked_seconds=total_seconds,
        ))

    return result


@router.get("/me/tasks", response_model=List[TaskWithAssignees])
def get_my_tasks(
    status: str = None,
    project_id: int = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get tasks assigned to the current user."""
    # Get task IDs assigned to user
    query = db.query(TaskAssignment).filter(TaskAssignment.user_id == current_user.id)
    assignments = query.all()
    task_ids = [a.task_id for a in assignments]

    # Build assignment lookup for is_primary
    assignment_lookup = {a.task_id: a for a in assignments}

    # Query tasks
    task_query = db.query(Task).filter(Task.id.in_(task_ids))
    if status:
        task_query = task_query.filter(Task.status == status)
    if project_id:
        task_query = task_query.filter(Task.project_id == project_id)

    tasks = task_query.all()

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

        # Get all assignees for this task
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


@router.get("/me/tasks/grouped", response_model=List[ProjectWithTasks])
def get_my_tasks_grouped(
    status: str = None,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get tasks assigned to the current user, grouped by project."""
    # Get task IDs assigned to user
    assignments = db.query(TaskAssignment).filter(TaskAssignment.user_id == current_user.id).all()
    task_ids = [a.task_id for a in assignments]
    assignment_lookup = {a.task_id: a for a in assignments}

    # Query tasks
    task_query = db.query(Task).filter(Task.id.in_(task_ids))
    if status:
        task_query = task_query.filter(Task.status == status)

    tasks = task_query.all()

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

        is_primary = assignment_lookup.get(task.id, {})
        is_primary_assignee = is_primary.is_primary if hasattr(is_primary, 'is_primary') else False

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
