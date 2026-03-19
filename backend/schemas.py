from pydantic import BaseModel
from datetime import datetime
from typing import Optional, List


# ============================================================================
# User schemas
# ============================================================================

class UserResponse(BaseModel):
    id: int
    email: str
    name: Optional[str]
    picture: Optional[str]
    is_admin: bool = False
    created_at: datetime

    class Config:
        from_attributes = True


class UserInfo(BaseModel):
    id: int
    email: str
    name: Optional[str]
    picture: Optional[str]

    class Config:
        from_attributes = True


class GoogleAuthRequest(BaseModel):
    token: str  # Google ID token from frontend


class AuthResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"
    user: UserResponse


# ============================================================================
# Project schemas
# ============================================================================

class ProjectCreate(BaseModel):
    name: str
    description: Optional[str] = None
    color: Optional[str] = None


class ProjectUpdate(BaseModel):
    name: Optional[str] = None
    description: Optional[str] = None
    status: Optional[str] = None  # active, archived, on_hold
    color: Optional[str] = None


class ProjectMemberInfo(BaseModel):
    id: int
    user_id: int
    user: UserInfo
    role: str  # member, lead
    allocated_at: datetime

    class Config:
        from_attributes = True


class ProjectResponse(BaseModel):
    id: int
    name: str
    description: Optional[str]
    status: str
    color: Optional[str]
    created_by: int
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


class ProjectWithDetails(ProjectResponse):
    creator: UserInfo
    member_count: int = 0
    task_count: int = 0
    total_tracked_seconds: int = 0


class ProjectWithMembers(ProjectResponse):
    members: List[ProjectMemberInfo] = []


class AddProjectMembersRequest(BaseModel):
    user_ids: List[int]
    role: str = "member"  # member or lead


# ============================================================================
# Task schemas
# ============================================================================

class TaskCreate(BaseModel):
    name: str
    max_hours: int
    description: Optional[str] = None
    priority: str = "medium"  # low, medium, high, urgent
    due_date: Optional[datetime] = None
    status: str = "todo"
    assignee_ids: Optional[List[int]] = None  # User IDs to assign task to (if empty, assigns to all project members)


class TaskUpdate(BaseModel):
    name: Optional[str] = None
    description: Optional[str] = None
    max_hours: Optional[int] = None
    priority: Optional[str] = None
    due_date: Optional[datetime] = None
    status: Optional[str] = None


class TaskAssigneeInfo(BaseModel):
    id: int
    user_id: int
    user: UserInfo
    is_primary: bool
    assigned_at: datetime

    class Config:
        from_attributes = True


class TaskResponse(BaseModel):
    id: int
    project_id: int
    name: str
    description: Optional[str]
    max_hours: int
    status: str = "todo"
    priority: str = "medium"
    due_date: Optional[datetime]
    created_by: int
    created_at: datetime
    updated_at: datetime

    class Config:
        from_attributes = True


class TaskWithTotalTime(TaskResponse):
    total_tracked_seconds: int
    remaining_seconds: int


class TaskWithAssignees(TaskWithTotalTime):
    assignees: List[TaskAssigneeInfo] = []
    project_name: Optional[str] = None
    project_color: Optional[str] = None


class TaskStatusUpdate(BaseModel):
    status: str  # todo, in_progress, review, done


class AssignTaskRequest(BaseModel):
    user_ids: List[int]
    primary_user_id: Optional[int] = None  # Which user is primary assignee


# ============================================================================
# TimeEntry schemas
# ============================================================================

class TimeEntryResponse(BaseModel):
    id: int
    task_id: int
    project_id: Optional[int]
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
    slot_seconds: int  # Duration of THIS sync slot (seconds tracked since last sync)
    session_start: datetime  # When the timer session started
    slot_end: datetime  # When this sync slot ends (current time or stop time)
    is_final: bool = False  # True if timer was stopped


class StopTimerRequest(BaseModel):
    client_stopped_at: Optional[datetime] = None


# ============================================================================
# Timer status
# ============================================================================

class TimerStatus(BaseModel):
    running: bool
    task: Optional[TaskResponse] = None
    time_entry_id: Optional[int] = None
    elapsed_seconds: Optional[int] = None


# ============================================================================
# Report schemas
# ============================================================================

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


# ============================================================================
# Daily Summary schemas
# ============================================================================

class TaskDailySummary(BaseModel):
    task_id: int
    task_name: str
    status: str
    total_seconds: int
    time_entries_count: int

    class Config:
        from_attributes = True


class UserDailySummary(BaseModel):
    date: str  # YYYY-MM-DD format
    user: UserInfo
    tasks: list[TaskDailySummary]
    total_work_seconds: int
    total_tasks_worked: int

    class Config:
        from_attributes = True


# ============================================================================
# Grouped responses for user endpoints
# ============================================================================

class TaskInProjectGroup(BaseModel):
    """Task info for grouping within a project"""
    id: int
    name: str
    description: Optional[str]
    max_hours: int
    status: str
    priority: str
    due_date: Optional[datetime]
    total_tracked_seconds: int
    remaining_seconds: int
    is_primary_assignee: bool

    class Config:
        from_attributes = True


class ProjectWithTasks(BaseModel):
    """Project with its assigned tasks grouped together"""
    id: int
    name: str
    description: Optional[str]
    status: str
    color: Optional[str]
    tasks: List[TaskInProjectGroup] = []

    class Config:
        from_attributes = True
