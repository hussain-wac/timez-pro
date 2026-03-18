from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from sqlalchemy import func
from datetime import datetime, timezone, timedelta
import logging

from database import get_db
from models import Task, TimeEntry, User
from schemas import (
    TaskCreate,
    TaskResponse,
    TaskWithTotalTime,
    TimeEntryResponse,
    SyncTimeRequest,
    CrashRecoveryRequest,
    CrashRecoveryResponse,
)
from auth import get_current_user

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/api/tasks", tags=["tasks"])


@router.get("", response_model=list[TaskWithTotalTime])
def list_tasks(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List all tasks for the current user with their total tracked time."""
    tasks = db.query(Task).filter(Task.user_id == current_user.id).all()
    
    result = []
    for task in tasks:
        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0
        
        max_seconds = task.max_hours * 3600
        remaining_seconds = max(0, max_seconds - total_seconds)
        
        result.append(TaskWithTotalTime(
            id=task.id,
            name=task.name,
            max_hours=task.max_hours,
            status=task.status,
            created_at=task.created_at,
            updated_at=task.updated_at,
            total_tracked_seconds=total_seconds,
            remaining_seconds=remaining_seconds,
        ))
    
    return result


@router.get("/timer", response_model=list[TaskWithTotalTime])
def list_timer_tasks(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """List tasks for timer - only in_progress status with total tracked time."""
    tasks = db.query(Task).filter(
        Task.user_id == current_user.id,
        Task.status == "in_progress"
    ).all()
    
    result = []
    for task in tasks:
        total_seconds = (
            db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
            .filter(TimeEntry.task_id == task.id, TimeEntry.duration.isnot(None))
            .scalar()
        ) or 0
        
        max_seconds = task.max_hours * 3600
        remaining_seconds = max(0, max_seconds - total_seconds)
        
        result.append(TaskWithTotalTime(
            id=task.id,
            name=task.name,
            max_hours=task.max_hours,
            status=task.status,
            created_at=task.created_at,
            updated_at=task.updated_at,
            total_tracked_seconds=total_seconds,
            remaining_seconds=remaining_seconds,
        ))
    
    return result


@router.post("", response_model=TaskResponse, status_code=201)
def create_task(
    task: TaskCreate,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Create a new task for the current user."""
    db_task = Task(name=task.name, max_hours=task.max_hours, user_id=current_user.id)
    db.add(db_task)
    db.commit()
    db.refresh(db_task)
    return db_task


@router.get("/{task_id}", response_model=TaskWithTotalTime)
def get_task(
    task_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Get a single task with its total tracked time."""
    task = db.query(Task).filter(
        Task.id == task_id, Task.user_id == current_user.id
    ).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    total_seconds = (
        db.query(func.coalesce(func.sum(TimeEntry.duration), 0))
        .filter(TimeEntry.task_id == task_id, TimeEntry.duration.isnot(None))
        .scalar()
    )

    max_seconds = task.max_hours * 3600
    remaining_seconds = max(0, max_seconds - total_seconds)

    return TaskWithTotalTime(
        id=task.id,
        name=task.name,
        max_hours=task.max_hours,
        status=task.status,
        created_at=task.created_at,
        updated_at=task.updated_at,
        total_tracked_seconds=total_seconds,
        remaining_seconds=remaining_seconds,
    )


@router.delete("/{task_id}", status_code=204)
def delete_task(
    task_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Delete a task and all its time entries."""
    task = db.query(Task).filter(
        Task.id == task_id, Task.user_id == current_user.id
    ).first()
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
    task = db.query(Task).filter(
        Task.id == task_id, Task.user_id == current_user.id
    ).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

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
    """Upsert time entry with cumulative time. Stores client timestamps for crash recovery."""
    task = db.query(Task).filter(
        Task.id == request.task_id, Task.user_id == current_user.id
    ).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

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
        running.client_started_at = request.client_started_at
        running.client_stopped_at = request.client_stopped_at
        running.duration = request.elapsed_seconds
        if request.client_stopped_at:
            running.end_time = request.client_stopped_at
            running.is_synced = True
        db.commit()
        db.refresh(running)
    else:
        entry = TimeEntry(
            task_id=request.task_id,
            user_id=current_user.id,
            start_time=request.client_started_at,
            end_time=request.client_stopped_at,
            duration=request.elapsed_seconds,
            client_started_at=request.client_started_at,
            client_stopped_at=request.client_stopped_at,
            is_synced=request.client_stopped_at is not None,
        )
        db.add(entry)
        db.commit()
        db.refresh(entry)
        running = entry

    # Update task status and user's current task
    if request.client_stopped_at:
        # Timer stopped - mark task as done and clear current task
        task.status = "done"
        current_user.current_task_id = None
    else:
        # Timer running - mark task as in_progress and set as current task
        task.status = "in_progress"
        current_user.current_task_id = request.task_id

    db.commit()
    db.refresh(task)

    return running


@router.post("/crash-recovery", response_model=CrashRecoveryResponse)
def crash_recovery(
    request: CrashRecoveryRequest,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Handle crash recovery for running time entries."""
    task = db.query(Task).filter(
        Task.id == request.task_id, Task.user_id == current_user.id
    ).first()
    if not task:
        raise HTTPException(status_code=404, detail="Task not found")

    running = (
        db.query(TimeEntry)
        .filter(
            TimeEntry.task_id == request.task_id,
            TimeEntry.user_id == current_user.id,
            TimeEntry.end_time.is_(None),
        )
        .first()
    )

    if not running:
        return CrashRecoveryResponse(
            success=True,
            message="No running entry found for this task",
            recovered_entry=None,
        )

    now = datetime.now(timezone.utc)
    time_since_stop = now - request.client_last_stopped_at.replace(tzinfo=timezone.utc)

    if time_since_stop > timedelta(minutes=15):
        discarded_duration = int(time_since_stop.total_seconds())
        logger.warning(
            f"CRASH DETECTED: Task {request.task_id}, user {current_user.id}. "
            f"Discarded {discarded_duration}s of untracked time. "
            f"client_started={running.client_started_at}, client_last_stopped={request.client_last_stopped_at}"
        )
        running.end_time = request.client_last_stopped_at
        running.client_stopped_at = request.client_last_stopped_at
        running.duration = int(
            (request.client_last_stopped_at - running.client_started_at).total_seconds()
        ) if running.client_started_at else discarded_duration
        running.is_synced = True
        db.commit()
        db.refresh(running)

        return CrashRecoveryResponse(
            success=True,
            message="Recovered entry by closing it at last known good stop time",
            recovered_entry=TimeEntryResponse.model_validate(running),
        )

    return CrashRecoveryResponse(
        success=True,
        message="Entry is still within acceptable range",
        recovered_entry=TimeEntryResponse.model_validate(running),
    )
