# Backend API Documentation

## Time Tracking Endpoints

### Sync Time
**POST** `/api/tasks/sync-time`

Frontend sends elapsed time, backend stores it.

**Request Body:**
```json
{
  "task_id": 1,
  "elapsed_seconds": 3600,
  "client_started_at": "2024-01-15T09:00:00",
  "client_stopped_at": "2024-01-15T10:00:00"
}
```

**Response:**
```json
{
  "id": 1,
  "task_id": 1,
  "user_id": 1,
  "start_time": "2024-01-15T09:00:00",
  "end_time": "2024-01-15T10:00:00",
  "duration": 3600,
  "client_started_at": "2024-01-15T09:00:00",
  "client_stopped_at": "2024-01-15T10:00:00",
  "is_synced": true
}
```

### Get Timer Status
**GET** `/api/status`

Since time is tracked by frontend, this always returns not running.

**Response:**
```json
{
  "running": false,
  "task": null,
  "time_entry_id": null,
  "elapsed_seconds": null
}
```

---

## Task Endpoints

### List Tasks
**GET** `/api/tasks`

Returns all tasks for the current user.

### Create Task
**POST** `/api/tasks`

**Request Body:**
```json
{
  "name": "Task Name",
  "max_hours": 40
}
```

### Get Task
**GET** `/api/tasks/{task_id}`

Returns a single task with total tracked time.

### Delete Task
**DELETE** `/api/tasks/{task_id}`

### Get Task Entries
**GET** `/api/tasks/{task_id}/entries`

---

## Dashboard Endpoints

### Get Dashboard Data
**GET** `/api/dashboard`

Returns stats, users, and tasks data.

### Get All Tasks with Time
**GET** `/api/dashboard/tasks`

Returns all tasks with total tracked time.

---

## Auth Endpoints

### Login
**POST** `/api/auth/login`

### Logout
**POST** `/api/auth/logout`

### Get Current User
**GET** `/api/auth/me`

---

## Notes

- Time tracking is handled entirely by the frontend
- Frontend sends elapsed time to `/api/tasks/sync-time` when user stops the timer
- `start_timer` and `stop_timer` endpoints are deprecated (no-op)
- Time entries are marked as `is_synced: true` when synced from frontend
