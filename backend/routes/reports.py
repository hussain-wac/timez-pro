from fastapi import APIRouter, Depends, Query
from sqlalchemy.orm import Session
from sqlalchemy import func
from datetime import datetime

from database import get_db
from models import Task, TimeEntry, User, TaskAssignment, Project
from schemas import DailyReport, SummaryReport, TaskTimeReport
from auth import get_current_user

router = APIRouter(prefix="/api/report", tags=["reports"])


@router.get("/daily", response_model=DailyReport)
def daily_report(
    date: str = Query(..., description="Date in YYYY-MM-DD format"),
    project_id: int = Query(None, description="Filter by project ID"),
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get total time per task for a given day. Only shows tasks assigned to the current user."""
    try:
        report_date = datetime.strptime(date, "%Y-%m-%d").date()
    except ValueError:
        report_date = datetime.now().date()

    # Get task IDs assigned to user
    task_ids_query = db.query(TaskAssignment.task_id).filter(
        TaskAssignment.user_id == current_user.id
    )
    task_ids = [t[0] for t in task_ids_query.all()]

    # Query completed time entries for the given day (user-specific)
    query = (
        db.query(
            Task.id,
            Task.name,
            func.coalesce(func.sum(TimeEntry.duration), 0).label("total_seconds"),
        )
        .join(TimeEntry, Task.id == TimeEntry.task_id)
        .filter(
            Task.id.in_(task_ids),
            func.date(TimeEntry.start_time) == report_date,
            TimeEntry.duration.isnot(None),
            TimeEntry.user_id == current_user.id,
        )
    )

    if project_id:
        query = query.filter(Task.project_id == project_id)

    results = query.group_by(Task.id, Task.name).all()

    tasks = [
        TaskTimeReport(task_id=r.id, task_name=r.name, total_seconds=r.total_seconds)
        for r in results
    ]

    total = sum(t.total_seconds for t in tasks)

    return DailyReport(date=date, tasks=tasks, total_seconds=total)


@router.get("/summary", response_model=SummaryReport)
def summary_report(
    project_id: int = Query(None, description="Filter by project ID"),
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get total time per task (all time). Only shows tasks assigned to the current user."""
    # Get task IDs assigned to user
    task_ids_query = db.query(TaskAssignment.task_id).filter(
        TaskAssignment.user_id == current_user.id
    )
    task_ids = [t[0] for t in task_ids_query.all()]

    query = (
        db.query(
            Task.id,
            Task.name,
            func.coalesce(func.sum(TimeEntry.duration), 0).label("total_seconds"),
        )
        .outerjoin(TimeEntry, (Task.id == TimeEntry.task_id) & (TimeEntry.user_id == current_user.id))
        .filter(Task.id.in_(task_ids))
    )

    if project_id:
        query = query.filter(Task.project_id == project_id)

    results = query.group_by(Task.id, Task.name).all()

    tasks = [
        TaskTimeReport(task_id=r.id, task_name=r.name, total_seconds=r.total_seconds)
        for r in results
    ]

    total = sum(t.total_seconds for t in tasks)

    return SummaryReport(tasks=tasks, total_seconds=total)


@router.get("/project/{project_id}", response_model=SummaryReport)
def project_report(
    project_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get total time per task for a project. Admin sees all, users see their own time."""
    project = db.query(Project).filter(Project.id == project_id).first()
    if not project:
        return SummaryReport(tasks=[], total_seconds=0)

    if current_user.is_admin:
        # Admin sees all tasks in project
        results = (
            db.query(
                Task.id,
                Task.name,
                func.coalesce(func.sum(TimeEntry.duration), 0).label("total_seconds"),
            )
            .outerjoin(TimeEntry, Task.id == TimeEntry.task_id)
            .filter(Task.project_id == project_id)
            .group_by(Task.id, Task.name)
            .all()
        )
    else:
        # Non-admin sees only their assigned tasks
        task_ids_query = db.query(TaskAssignment.task_id).filter(
            TaskAssignment.user_id == current_user.id
        )
        task_ids = [t[0] for t in task_ids_query.all()]

        results = (
            db.query(
                Task.id,
                Task.name,
                func.coalesce(func.sum(TimeEntry.duration), 0).label("total_seconds"),
            )
            .outerjoin(TimeEntry, (Task.id == TimeEntry.task_id) & (TimeEntry.user_id == current_user.id))
            .filter(Task.project_id == project_id, Task.id.in_(task_ids))
            .group_by(Task.id, Task.name)
            .all()
        )

    tasks = [
        TaskTimeReport(task_id=r.id, task_name=r.name, total_seconds=r.total_seconds)
        for r in results
    ]

    total = sum(t.total_seconds for t in tasks)

    return SummaryReport(tasks=tasks, total_seconds=total)
