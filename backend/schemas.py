from pydantic import BaseModel
from datetime import datetime
from typing import Optional


# User schemas
class UserResponse(BaseModel):
    id: int
    email: str
    name: Optional[str]
    picture: Optional[str]
    is_admin: bool = False
    created_at: datetime

    class Config:
        from_attributes = True


class GoogleAuthRequest(BaseModel):
    token: str  # Google ID token from frontend


class AuthResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"
    user: UserResponse


# Task schemas
class TaskCreate(BaseModel):
    name: str
    max_hours: int
    user_id: Optional[int] = None
    status: str = "todo"


class TaskResponse(BaseModel):
    id: int
    name: str
    max_hours: int
    status: str = "todo"
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


class TaskWithTotalTime(TaskResponse):
    total_tracked_seconds: int
    remaining_seconds: int
    status: str = "todo"


class TaskStatusUpdate(BaseModel):
    status: str  # todo, in_progress, review, done


# TimeEntry schemas
class TimeEntryResponse(BaseModel):
    id: int
    task_id: int
    start_time: datetime
    end_time: Optional[datetime]
    duration: Optional[int]
    created_at: datetime
    client_started_at: Optional[datetime] = None
    client_stopped_at: Optional[datetime] = None
    is_synced: bool = False

    class Config:
        from_attributes = True


class SyncTimeRequest(BaseModel):
    task_id: int
    elapsed_seconds: int
    client_started_at: datetime
    client_stopped_at: Optional[datetime] = None


class CrashRecoveryRequest(BaseModel):
    task_id: int
    client_last_stopped_at: datetime


class CrashRecoveryResponse(BaseModel):
    success: bool
    message: str
    recovered_entry: Optional[TimeEntryResponse] = None


class StopTimerRequest(BaseModel):
    client_stopped_at: Optional[datetime] = None


# Timer status
class TimerStatus(BaseModel):
    running: bool
    task: Optional[TaskResponse] = None
    time_entry_id: Optional[int] = None
    elapsed_seconds: Optional[int] = None


# Report schemas
class TaskTimeReport(BaseModel):
    task_id: int
    task_name: str
    total_seconds: int


class DailyReport(BaseModel):
    date: str
    tasks: list[TaskTimeReport]
    total_seconds: int


class SummaryReport(BaseModel):
    tasks: list[TaskTimeReport]
    total_seconds: int


class EmployeeWithHours(BaseModel):
    id: int
    email: str
    name: Optional[str]
    picture: Optional[str]
    monthly_hours: int = 0
    task_count: int = 0
