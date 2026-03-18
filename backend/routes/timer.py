# NOTE: The /api/tasks/sync-time endpoint was previously duplicated here.
# It has been removed because the canonical implementation lives in tasks.py.
# Since tasks.router is registered before timer.router in main.py,
# the tasks.py implementation takes precedence. Having duplicate endpoints
# caused confusion and the timer.py version was never actually called.
#
# The sync-time endpoint in tasks.py now handles:
# - Upserting time entries with cumulative time
# - Updating task.status to "in_progress" when timer is running
# - Updating task.status to "done" when timer stops
# - Updating user.current_task_id appropriately
#
# If you need timer-related functionality, add it to tasks.py or create
# new endpoints with unique paths here.

from fastapi import APIRouter

router = APIRouter(prefix="/api", tags=["timer"])

# Timer-specific endpoints can be added here with unique paths
# (not conflicting with tasks.py routes)
