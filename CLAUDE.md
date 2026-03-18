# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Timez Pro is a time-tracking application with three main components:
- **backend**: FastAPI (Python) REST API server
- **dashboard**: React admin dashboard (managers view)
- **desktop-app**: Tauri + React desktop application (employee time tracker)

## Commands

### Backend (Python/FastAPI)
```bash
cd backend
source venv/bin/activate
pip install -r requirements.txt      # Install dependencies
uvicorn main:app --reload            # Run dev server (port 8000)
```

### Dashboard (React/Vite)
```bash
cd dashboard
npm install
npm run dev       # Run dev server (port 5173)
npm run build     # Production build
npm run lint      # ESLint
```

### Desktop App (Tauri)
```bash
cd desktop-app
npm install
npm run dev                          # Run Vite dev server only
npm run tauri dev                    # Run full Tauri app in dev mode
npm run tauri build                  # Production build
npm run tauri:prepare-service        # Build service binaries
```

#### Building Rust services
```bash
cd desktop-app/src-tauri
cargo build                          # Build main app
cargo build -p timez-service         # Build service crate
```

## Architecture

### Backend (FastAPI)
- `main.py` - App entry, CORS config, router registration
- `models.py` - SQLAlchemy models: User, Task, TimeEntry
- `schemas.py` - Pydantic schemas
- `auth.py` - Google OAuth authentication
- `database.py` - SQLite connection (timetracker.db)
- `routes/` - API endpoints (auth, tasks, timer, reports, dashboard)

### Desktop App (Tauri)
The desktop app uses a **service architecture** with separate processes communicating via Unix domain sockets:

**Rust Workspace** (`desktop-app/src-tauri/`):
- `app/` - Main Tauri application with tray icon, window management
- `core/` - Shared types: models, protocol (IPC Request/Response enums), API client, idle detection
- `service/` - Background service binaries:
  - `timez-auth-service` - Authentication state
  - `timez-task-service` - Task management
  - `timez-tracker-service` - Timer/tracking logic
  - `timez-idle-time-service` - Idle detection
  - `timez-quit-service` - Graceful shutdown

**IPC Protocol** (`core/src/protocol.rs`):
- `Request` enum: ListTasks, StartTimer, StopTimer, GetStatus, etc.
- `ResponseData` enum: Tasks, Status, Activity, AuthResponse, etc.
- Services communicate via JSON over Unix sockets at `/tmp/timez-*.sock`

**Frontend** (`desktop-app/src/`):
- React + TypeScript + Tailwind
- Uses `@tauri-apps/api` to invoke Rust commands
- Listens for events: `idle-detected`, `timer-stopped`, `request-quit-confirm`

### Dashboard (React)
- Admin-only interface for viewing employee time data
- Routes: `/` (home), `/employees`, `/employees/:userId` (kanban)
- Uses Google OAuth, requires `is_admin: true`

## Key Data Flow

1. Time tracking happens client-side in the desktop app
2. Desktop app syncs elapsed time to backend via `POST /api/tasks/sync-time`
3. Backend stores TimeEntry records with client timestamps
4. Dashboard fetches aggregated data from `/api/dashboard` endpoints

## Database

SQLite at `backend/timetracker.db` with tables:
- `users` - email, name, google_id, is_admin, current_task_id
- `tasks` - user_id, name, max_hours, status (todo/in_progress/review/done)
- `time_entries` - task_id, user_id, start_time, end_time, duration, is_synced

## Code Quality Agents

Use these specialized agents for code review and architecture guidance:

### rust-quality-guardian
Use for Rust code in `desktop-app/src-tauri/`. Invoke for:
- Memory safety and ownership review
- Performance optimization
- Idiomatic Rust patterns
- Thread safety and concurrency
- Lifetime annotations and clone elimination

### react-ui-architect
Use for React code in `dashboard/` and `desktop-app/src/`. Invoke for:
- Component architecture and decomposition
- Performance optimization (re-renders, memoization)
- State management patterns
- Accessibility compliance
- Production-grade React patterns
