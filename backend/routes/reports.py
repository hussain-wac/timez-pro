from fastapi import APIRouter, Depends, Query
from sqlalchemy.orm import Session
from sqlalchemy import func
from datetime import datetime

from database import get_db
from models import Task, TimeEntry, User
from schemas import DailyReport, SummaryReport, TaskTimeReport
from auth import get_current_user

router = APIRouter(prefix="/api/report", tags=["reports"])


@router.get("/daily", response_model=DailyReport)
def daily_report(
    date: str = Query(..., description="Date in YYYY-MM-DD format"),
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get total time per task for a given day."""
    try:
        report_date = datetime.strptime(date, "%Y-%m-%d").date()
    except ValueError:
        report_date = datetime.now().date()

    # Query completed time entries for the given day (user-specific)
    results = (
        db.query(
            Task.id,
            Task.name,
            func.coalesce(func.sum(TimeEntry.duration), 0).label("total_seconds"),
        )
        .join(TimeEntry, Task.id == TimeEntry.task_id)
        .filter(
            Task.user_id == current_user.id,
            func.date(TimeEntry.start_time) == report_date,
            TimeEntry.duration.isnot(None),
        )
        .group_by(Task.id, Task.name)
        .all()
    )

    tasks = [
        TaskTimeReport(task_id=r.id, task_name=r.name, total_seconds=r.total_seconds)
        for r in results
    ]

    total = sum(t.total_seconds for t in tasks)

    return DailyReport(date=date, tasks=tasks, total_seconds=total)


@router.get("/summary", response_model=SummaryReport)
def summary_report(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get total time per task (all time)."""
    results = (
        db.query(
            Task.id,
            Task.name,
            func.coalesce(func.sum(TimeEntry.duration), 0).label("total_seconds"),
        )
        .outerjoin(TimeEntry, Task.id == TimeEntry.task_id)
        .filter(Task.user_id == current_user.id)
        .group_by(Task.id, Task.name)
        .all()
    )

    tasks = [
        TaskTimeReport(task_id=r.id, task_name=r.name, total_seconds=r.total_seconds)
        for r in results
    ]

    total = sum(t.total_seconds for t in tasks)

    return SummaryReport(tasks=tasks, total_seconds=total)
