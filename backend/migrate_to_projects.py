#!/usr/bin/env python3
"""
Migration script to transform from user-owned tasks to project-based architecture.

This script:
1. Creates new tables: projects, project_members, task_assignments
2. Creates a default project for existing tasks
3. Migrates existing tasks to the new project
4. Creates task assignments from existing user_id relationships
5. Updates time_entries with project_id

Run this script after updating models.py but before running the application:
    python migrate_to_projects.py

NOTE: This is a one-time migration. Backup your database before running!
"""

import sqlite3
from datetime import datetime


def migrate():
    conn = sqlite3.connect("timetracker.db")
    cursor = conn.cursor()

    print("Starting migration to project-based architecture...")

    # Check if migration is needed by looking for projects table
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='projects'")
    if cursor.fetchone():
        print("Projects table already exists. Checking for additional migrations...")
    else:
        print("Creating projects table...")
        cursor.execute("""
            CREATE TABLE projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR NOT NULL,
                description TEXT,
                status VARCHAR DEFAULT 'active',
                color VARCHAR,
                created_by INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (created_by) REFERENCES users(id)
            )
        """)
        cursor.execute("CREATE INDEX ix_projects_id ON projects(id)")

    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='project_members'")
    if not cursor.fetchone():
        print("Creating project_members table...")
        cursor.execute("""
            CREATE TABLE project_members (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                role VARCHAR DEFAULT 'member',
                allocated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects(id),
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
        """)
        cursor.execute("CREATE INDEX ix_project_members_id ON project_members(id)")

    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='task_assignments'")
    if not cursor.fetchone():
        print("Creating task_assignments table...")
        cursor.execute("""
            CREATE TABLE task_assignments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                assigned_by INTEGER NOT NULL,
                assigned_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                is_primary BOOLEAN DEFAULT 0,
                FOREIGN KEY (task_id) REFERENCES tasks(id),
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (assigned_by) REFERENCES users(id)
            )
        """)
        cursor.execute("CREATE INDEX ix_task_assignments_id ON task_assignments(id)")

    # Check if tasks table needs to be modified
    cursor.execute("PRAGMA table_info(tasks)")
    columns = {row[1] for row in cursor.fetchall()}

    if "project_id" not in columns:
        print("Adding new columns to tasks table...")

        # Add new columns to tasks
        cursor.execute("ALTER TABLE tasks ADD COLUMN project_id INTEGER")
        cursor.execute("ALTER TABLE tasks ADD COLUMN description TEXT")
        cursor.execute("ALTER TABLE tasks ADD COLUMN priority VARCHAR DEFAULT 'medium'")
        cursor.execute("ALTER TABLE tasks ADD COLUMN due_date DATETIME")
        cursor.execute("ALTER TABLE tasks ADD COLUMN created_by INTEGER")

    # Check if time_entries needs project_id
    cursor.execute("PRAGMA table_info(time_entries)")
    te_columns = {row[1] for row in cursor.fetchall()}

    if "project_id" not in te_columns:
        print("Adding project_id to time_entries table...")
        cursor.execute("ALTER TABLE time_entries ADD COLUMN project_id INTEGER")

    # Check if there are existing tasks without project_id
    cursor.execute("SELECT COUNT(*) FROM tasks WHERE project_id IS NULL")
    tasks_without_project = cursor.fetchone()[0]

    if tasks_without_project > 0:
        print(f"Found {tasks_without_project} tasks without project_id. Creating default project...")

        # Get an admin user to be the creator
        cursor.execute("SELECT id FROM users WHERE is_admin = 1 LIMIT 1")
        admin_row = cursor.fetchone()
        if not admin_row:
            cursor.execute("SELECT id FROM users LIMIT 1")
            admin_row = cursor.fetchone()

        if admin_row:
            admin_id = admin_row[0]

            # Create default project
            cursor.execute("""
                INSERT INTO projects (name, description, status, color, created_by, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            """, (
                "Default Project",
                "Migrated tasks from previous version",
                "active",
                "#6366f1",  # Indigo color
                admin_id,
                datetime.now().isoformat(),
                datetime.now().isoformat(),
            ))
            default_project_id = cursor.lastrowid
            print(f"Created default project with ID: {default_project_id}")

            # Update all tasks without project_id
            cursor.execute("""
                UPDATE tasks SET project_id = ?, created_by = COALESCE(created_by, user_id)
                WHERE project_id IS NULL
            """, (default_project_id,))
            print(f"Updated {cursor.rowcount} tasks with default project")

            # Get all unique user_ids from tasks
            cursor.execute("SELECT DISTINCT user_id FROM tasks WHERE user_id IS NOT NULL")
            user_ids = [row[0] for row in cursor.fetchall()]

            # Add users as project members
            for user_id in user_ids:
                cursor.execute("""
                    INSERT INTO project_members (project_id, user_id, role, allocated_at)
                    VALUES (?, ?, ?, ?)
                """, (default_project_id, user_id, "member", datetime.now().isoformat()))
            print(f"Added {len(user_ids)} users as project members")

            # Create task assignments from existing user_id
            cursor.execute("""
                SELECT id, user_id FROM tasks WHERE user_id IS NOT NULL
            """)
            tasks = cursor.fetchall()

            for task_id, user_id in tasks:
                cursor.execute("""
                    INSERT INTO task_assignments (task_id, user_id, assigned_by, assigned_at, is_primary)
                    VALUES (?, ?, ?, ?, ?)
                """, (task_id, user_id, admin_id, datetime.now().isoformat(), True))
            print(f"Created {len(tasks)} task assignments")

            # Update time_entries with project_id
            cursor.execute("""
                UPDATE time_entries
                SET project_id = (SELECT project_id FROM tasks WHERE tasks.id = time_entries.task_id)
                WHERE project_id IS NULL
            """)
            print(f"Updated {cursor.rowcount} time entries with project_id")
        else:
            print("WARNING: No users found. Cannot create default project.")

    conn.commit()
    conn.close()

    print("\nMigration completed successfully!")
    print("\nNOTE: The 'user_id' column in tasks table is no longer used.")
    print("Task ownership is now determined by task_assignments table.")
    print("\nYou can now run the application with: uvicorn main:app --reload")


if __name__ == "__main__":
    migrate()
