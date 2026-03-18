from sqlalchemy import Column, Integer, String, DateTime, ForeignKey, Boolean
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func
from datetime import datetime

from database import Base


class User(Base):
    __tablename__ = "users"

    id = Column(Integer, primary_key=True, index=True)
    email = Column(String, unique=True, index=True, nullable=False)
    name = Column(String, nullable=True)
    picture = Column(String, nullable=True)
    google_id = Column(String, unique=True, index=True, nullable=False)
    is_admin = Column(Boolean, default=False)
    current_task_id = Column(Integer, nullable=True)
    created_at = Column(DateTime, server_default=func.now())
    updated_at = Column(DateTime, server_default=func.now(), onupdate=func.now())

    tasks = relationship("Task", back_populates="user", cascade="all, delete-orphan")
    time_entries = relationship("TimeEntry", back_populates="user", cascade="all, delete-orphan")


class Task(Base):
    __tablename__ = "tasks"

    id = Column(Integer, primary_key=True, index=True)
    user_id = Column(Integer, ForeignKey("users.id"), nullable=False)
    name = Column(String, nullable=False)
    max_hours = Column(Integer, nullable=False)
    status = Column(String, default="todo")  # todo, in_progress, review, done
    created_at = Column(DateTime, server_default=func.now())
    updated_at = Column(DateTime, server_default=func.now(), onupdate=func.now())

    user = relationship("User", back_populates="tasks")
    time_entries = relationship("TimeEntry", back_populates="task", cascade="all, delete-orphan")


class TimeEntry(Base):
    __tablename__ = "time_entries"

    id = Column(Integer, primary_key=True, index=True)
    task_id = Column(Integer, ForeignKey("tasks.id"), nullable=False)
    user_id = Column(Integer, ForeignKey("users.id"), nullable=False)
    start_time = Column(DateTime, nullable=False)
    end_time = Column(DateTime, nullable=True)
    duration = Column(Integer, nullable=True)  # seconds
    created_at = Column(DateTime, server_default=func.now())
    client_started_at = Column(DateTime, nullable=True)
    client_stopped_at = Column(DateTime, nullable=True)
    is_synced = Column(Boolean, default=False)

    task = relationship("Task", back_populates="time_entries")
    user = relationship("User", back_populates="time_entries")
