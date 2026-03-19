import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { useAuth } from "./AuthContext";

interface IdleEvent {
  idle_duration_secs: number;
  task_id: number;
  task_name: string;
  tracking_active: boolean;
}

interface Project {
  id: number;
  name: string;
  color: string | null;
  task_count: number;
}

interface Task {
  id: number;
  name: string;
  budget_secs: number;
  elapsed_secs: number;
  running: boolean;
  project_id: number | null;
  project_name: string | null;
}

const EIGHT_HOURS = 8 * 60 * 60;

function formatHms(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return `${h}:${m.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`;
}

function formatIdleDuration(secs: number): string {
  const mins = Math.floor(secs / 60);
  if (mins < 1) return `${secs} sec`;
  if (mins === 1) return "1 minute";
  const h = Math.floor(mins / 60);
  const remainMins = mins % 60;
  if (h > 0) return `${h}h ${remainMins}m`;
  return `${mins} minutes`;
}


// Helper to send desktop notification
async function showDesktopNotification(title: string, body: string) {
  try {
    let permissionGranted = await isPermissionGranted();
    if (!permissionGranted) {
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }
    if (permissionGranted) {
      sendNotification({ title, body });
    }
  } catch (e) {
    console.error("Failed to send notification:", e);
  }
}

function App() {
  const { user, logout } = useAuth();
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<number | null>(null);
  const [selectedTaskId, setSelectedTaskId] = useState<number | null>(null);
  const [projectSearch, setProjectSearch] = useState("");
  const [taskSearch, setTaskSearch] = useState("");
  const [idleEvent, setIdleEvent] = useState<IdleEvent | null>(null);
  const [idleTaskId, setIdleTaskId] = useState<number | null>(null);
  const [quitConfirmOpen, setQuitConfirmOpen] = useState(false);
  const [quitHasRunning, setQuitHasRunning] = useState(false);
  const [quitError, setQuitError] = useState<string | null>(null);
  const [crashRecoveryOpen, setCrashRecoveryOpen] = useState(false);
  const [crashRecoveredTaskId, setCrashRecoveredTaskId] = useState<number | null>(null);
  const [syncNotification, setSyncNotification] = useState<string | null>(null);

  // Track timer running state
  const [isTimerRunning, setIsTimerRunning] = useState(false);
  const isTimerRunningRef = useRef(false); // Ref for use in interval

  // Refs for drift-free timer calculation
  const timerStartedAtRef = useRef<number | null>(null); // Timestamp when timer started (ms)
  const baseElapsedRef = useRef(0); // Task elapsed seconds at start point
  const baseDailyTotalRef = useRef(0); // Daily total at start point

  // Daily total timer - saved locally by date, runs like a stopwatch
  const [dailyTotal, setDailyTotal] = useState(0);

  // Get today's date key for localStorage
  const getTodayKey = () => {
    const today = new Date();
    return `timez_daily_${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`;
  };

  // Load daily total from localStorage on mount
  useEffect(() => {
    const key = getTodayKey();
    const saved = localStorage.getItem(key);
    if (saved) {
      const parsed = parseInt(saved, 10);
      if (!isNaN(parsed)) {
        setDailyTotal(parsed);
        baseDailyTotalRef.current = parsed;
      }
    }
  }, []);

  // Save daily total to localStorage whenever it changes (debounced)
  useEffect(() => {
    const key = getTodayKey();
    localStorage.setItem(key, String(dailyTotal));
  }, [dailyTotal]);

  // Check for midnight reset
  useEffect(() => {
    const checkMidnight = () => {
      const now = new Date();
      if (now.getHours() === 0 && now.getMinutes() === 0) {
        // It's midnight - reset daily total
        setDailyTotal(0);
        baseDailyTotalRef.current = 0;
        // Clean up old date keys (keep only last 7 days)
        const keysToKeep = new Set<string>();
        for (let i = 0; i < 7; i++) {
          const d = new Date();
          d.setDate(d.getDate() - i);
          keysToKeep.add(`timez_daily_${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`);
        }
        for (let i = 0; i < localStorage.length; i++) {
          const k = localStorage.key(i);
          if (k && k.startsWith('timez_daily_') && !keysToKeep.has(k)) {
            localStorage.removeItem(k);
          }
        }
      }
    };

    // Check every minute
    const id = setInterval(checkMidnight, 60000);
    return () => clearInterval(id);
  }, []);

  const refreshProjects = useCallback(async () => {
    try {
      const result = await invoke<Project[]>("list_projects");
      setProjects(result);
      // Auto-select first project if none selected
      if (result.length > 0 && selectedProjectId === null) {
        setSelectedProjectId(result[0].id);
      }
    } catch {
      // API may be unreachable
    }
  }, [selectedProjectId]);

  // Check if any task is running globally (used on init)
  const checkRunningTask = useCallback(async () => {
    try {
      const result = await invoke<Task[]>("list_tasks");
      if (result) {
        const runningTask = result.find((t) => t.running);
        if (runningTask) {
          setIsTimerRunning(true);
          // Initialize refs if not already tracking
          if (!timerStartedAtRef.current) {
            timerStartedAtRef.current = Date.now();
            baseElapsedRef.current = runningTask.elapsed_secs;
            baseDailyTotalRef.current = dailyTotal;
          }
        } else {
          setIsTimerRunning(false);
          timerStartedAtRef.current = null;
        }
      }
    } catch (error) {
      console.error("Failed to check running task:", error);
    }
  }, [dailyTotal]);

  const refreshTasks = useCallback(async () => {
    try {
      let result: Task[] | null = null;

      if (selectedProjectId !== null) {
        result = await invoke<Task[]>("list_project_tasks", { projectId: selectedProjectId });
      } else {
        result = await invoke<Task[]>("list_tasks");
      }

      if (result) {
        setTasks(result);

        // Check if any task in CURRENT view is running
        const runningTaskInView = result.find((t) => t.running);

        if (runningTaskInView) {
          setIsTimerRunning(true);
          // Only initialize refs if not already tracking (avoid resetting mid-tick)
          if (!timerStartedAtRef.current) {
            timerStartedAtRef.current = Date.now();
            baseElapsedRef.current = runningTaskInView.elapsed_secs;
            baseDailyTotalRef.current = dailyTotal;
          }
        }
      }

      // Check globally if any task is running (for when viewing different project)
      if (selectedProjectId !== null) {
        const allTasks = await invoke<Task[]>("list_tasks");
        const globalRunningTask = allTasks?.find((t) => t.running);
        if (globalRunningTask) {
          setIsTimerRunning(true);
          if (!timerStartedAtRef.current) {
            timerStartedAtRef.current = Date.now();
            baseElapsedRef.current = globalRunningTask.elapsed_secs;
            baseDailyTotalRef.current = dailyTotal;
          }
        }
      }
    } catch (error) {
      console.error("Failed to refresh tasks:", error);
    }
  }, [selectedProjectId, dailyTotal]);

  // Initial load
  useEffect(() => {
    refreshProjects();
    checkRunningTask();
  }, []);

  // Refresh tasks when project changes
  useEffect(() => {
    refreshTasks();
  }, [selectedProjectId]);

  // Check for idle events
  useEffect(() => {
    const id = setInterval(async () => {
      try {
        const event = await invoke<IdleEvent | null>("get_idle_event");
        setIdleEvent((prev) => {
          if (!event) return null;
          if (prev && prev.task_id === event.task_id &&
              prev.idle_duration_secs === event.idle_duration_secs &&
              prev.tracking_active === event.tracking_active) {
            return prev;
          }
          return event;
        });
        if (event) {
          setIdleTaskId((prev) => prev ?? event.task_id);
        }
      } catch {
        // backend may be restarting
      }
    }, 1000);
    return () => clearInterval(id);
  }, []);

  // Sync tasks with backend every 30 seconds
  useEffect(() => {
    const id = setInterval(() => {
      // Only sync when timer is NOT running to prevent value jumps
      if (!isTimerRunningRef.current) {
        refreshTasks();
      }
    }, 30000);
    return () => clearInterval(id);
  }, [refreshTasks]);

  // Keep ref in sync with state
  useEffect(() => {
    isTimerRunningRef.current = isTimerRunning;
  }, [isTimerRunning]);

  // Local tick for smooth real-time display - uses timestamp-based calculation to prevent drift
  useEffect(() => {
    const id = setInterval(() => {
      // Use ref to check running state (avoids recreating interval)
      if (!isTimerRunningRef.current || !timerStartedAtRef.current) return;

      // Calculate actual elapsed time from timestamp - this prevents drift
      const now = Date.now();
      const elapsedSinceStart = Math.floor((now - timerStartedAtRef.current) / 1000);

      // Protect against clock changes: if elapsed is negative or unreasonably large (> 1 hour jump), ignore
      if (elapsedSinceStart < 0 || elapsedSinceStart > 3600) {
        // Clock may have changed - reset the start timestamp
        timerStartedAtRef.current = now;
        return;
      }

      const newTaskElapsed = baseElapsedRef.current + elapsedSinceStart;
      const newDailyTotal = baseDailyTotalRef.current + elapsedSinceStart;

      // Update running task's elapsed time in the current view
      setTasks((prev) =>
        prev.map((t) =>
          t.running ? { ...t, elapsed_secs: newTaskElapsed } : t
        )
      );

      // Update daily total (stopwatch style)
      setDailyTotal(newDailyTotal);
    }, 1000);
    return () => clearInterval(id);
  }, []); // Empty deps - interval created once

  // Listen for events from Rust backend
  useEffect(() => {
    const unlisten1 = listen<IdleEvent>("idle-detected", (event) => {
      setIdleEvent(event.payload);
      setIdleTaskId((prev) => prev ?? event.payload.task_id);
      refreshTasks();
    });

    const unlisten2 = listen("timer-stopped", () => {
      refreshTasks();
    });

    const unlisten4 = listen<boolean>("request-quit-confirm", (event) => {
      setQuitError(null);
      setQuitHasRunning(!!event.payload);
      setQuitConfirmOpen(true);
    });

    const unlisten5 = listen<{ task_id: number; action?: string }>("crash-recovery-complete", (event) => {
      setCrashRecoveredTaskId(event.payload.task_id);
      setCrashRecoveryOpen(true);
    });

    const unlisten6 = listen<{ task_id: number; reason: string }>("time-discarded", (event) => {
      setCrashRecoveredTaskId(event.payload.task_id);
      setCrashRecoveryOpen(true);
    });

    const unlistenSync = listen<{ message?: string; syncing_seconds?: number; task_id?: number }>("sync-in-progress", (event) => {
      const msg = event.payload?.message || "Time sync in progress...";
      setSyncNotification(msg);
      showDesktopNotification("Timez Pro - Syncing", msg);
      setTimeout(() => setSyncNotification(null), 4000);
    });

    const unlistenSyncComplete = listen<{ message?: string; synced_seconds?: number; total_seconds?: number }>("sync-complete", (event) => {
      const msg = event.payload?.message || "Sync complete";
      setSyncNotification(msg);
      showDesktopNotification("Timez Pro - Synced", msg);
      refreshTasks(); // Refresh tasks after successful sync
      setTimeout(() => setSyncNotification(null), 3000);
    });

    const unlistenSyncError = listen<{ error?: string }>("sync-error", (event) => {
      const errorMsg = `Sync failed: ${event.payload?.error || "Unknown error"}`;
      setSyncNotification(errorMsg);
      showDesktopNotification("Timez Pro - Error", errorMsg);
      setTimeout(() => setSyncNotification(null), 5000);
    });

    const unlistenMidnight = listen("midnight-reset", () => {
      refreshTasks();
      setSyncNotification("Timer reset at midnight");
      showDesktopNotification("Timez Pro", "Timer reset at midnight");
      setTimeout(() => setSyncNotification(null), 5000);
    });

    // Listen for notifications from Rust backend
    const unlistenShowNotification = listen<{ title: string; body: string }>("show-notification", (event) => {
      showDesktopNotification(event.payload.title, event.payload.body);
    });

    return () => {
      unlisten1.then((fn) => fn());
      unlisten2.then((fn) => fn());
      unlisten4.then((fn) => fn());
      unlisten5.then((fn) => fn());
      unlisten6.then((fn) => fn());
      unlistenSync.then((fn) => fn());
      unlistenSyncComplete.then((fn) => fn());
      unlistenSyncError.then((fn) => fn());
      unlistenMidnight.then((fn) => fn());
      unlistenShowNotification.then((fn) => fn());
    };
  }, [refreshTasks]);

  const toggleTimer = async (taskId: number) => {
    const task = tasks.find((t) => t.id === taskId);
    try {
      if (task?.running) {
        // Stop the timer
        await invoke<Task[]>("stop_timer");

        // Save the current daily total before clearing refs
        // (dailyTotal state already has the correct value from last tick)
        baseDailyTotalRef.current = dailyTotal;

        // Clear timer refs
        timerStartedAtRef.current = null;
        setIsTimerRunning(false);

        // Update local state immediately for responsive UI
        setTasks((prev) =>
          prev.map((t) => (t.id === taskId ? { ...t, running: false } : t))
        );

        // Refresh to get accurate task values after sync
        await refreshTasks();
      } else {
        // Start the timer - this may stop another running timer
        // The backend handles stopping any existing timer and syncing
        await invoke<Task[]>("start_timer", { taskId });

        // Fetch task to get accurate elapsed value after backend sync
        const allTasks = await invoke<Task[]>("list_tasks");
        const newTaskFromServer = allTasks.find((t) => t.id === taskId);
        const currentElapsed = newTaskFromServer?.elapsed_secs ?? 0;

        // Initialize refs for drift-free tracking
        timerStartedAtRef.current = Date.now();
        baseElapsedRef.current = currentElapsed;
        baseDailyTotalRef.current = dailyTotal; // Continue from current daily total

        // Update current view tasks
        setTasks((prev) =>
          prev.map((t) => ({
            ...t,
            running: t.id === taskId,
          }))
        );
        setIsTimerRunning(true);
      }
      setSelectedTaskId(taskId);
    } catch (error) {
      console.error("Timer toggle failed:", error);
      // On error, refresh to get correct state and reset refs
      timerStartedAtRef.current = null;
      await refreshTasks();
    }
  };

  const handleConfirmQuit = async () => {
    setQuitError(null);
    if (quitHasRunning) {
      try {
        await invoke<Task[]>("stop_timer");
      } catch {
        setQuitError("Failed to stop the running timer. Please try again.");
        return;
      }
    }
    await invoke("quit_app");
  };

  const handleKeepIdleTime = async () => {
    if (!idleEvent || idleEvent.tracking_active || !idleTaskId) return;
    try {
      await invoke<Task[]>("add_idle_time", {
        taskId: idleTaskId,
        durationSecs: idleEvent.idle_duration_secs,
      });
      // Add idle time to daily total
      setDailyTotal((prev) => prev + idleEvent.idle_duration_secs);
      baseDailyTotalRef.current = dailyTotal + idleEvent.idle_duration_secs;
      // Refresh to get updated state
      await refreshTasks();
      await invoke("resolve_idle_event");
      setIdleEvent(null);
      setIdleTaskId(null);
    } catch (error) {
      console.error("Keep idle time failed:", error);
      await refreshTasks();
    }
  };

  const handleDiscardIdleTime = async () => {
    if (!idleEvent || idleEvent.tracking_active) return;
    try {
      await invoke<Task[]>("discard_idle_time", {
        taskId: idleEvent.task_id,
      });
      // Subtract idle time from daily total (it was already counted while idle)
      setDailyTotal((prev) => Math.max(0, prev - idleEvent.idle_duration_secs));
      baseDailyTotalRef.current = Math.max(0, dailyTotal - idleEvent.idle_duration_secs);
      // Refresh to get updated state
      await refreshTasks();
      await invoke("resolve_idle_event");
      setIdleEvent(null);
      setIdleTaskId(null);
    } catch (error) {
      console.error("Discard idle time failed:", error);
      await refreshTasks();
    }
  };

  const filteredProjects = projects.filter((p) =>
    p.name.toLowerCase().includes(projectSearch.toLowerCase()),
  );

  const filteredTasks = tasks.filter((t) =>
    t.name.toLowerCase().includes(taskSearch.toLowerCase()),
  );

  const selectedProject = projects.find((p) => p.id === selectedProjectId);
  const selectedTask = tasks.find((t) => t.id === selectedTaskId);
  // Today's total - runs like a stopwatch, saved locally by date
  const totalDaySeconds = dailyTotal;
  const overtime = totalDaySeconds > EIGHT_HOURS ? totalDaySeconds - EIGHT_HOURS : 0;

  return (
    <div className="h-screen flex bg-gray-100 text-gray-800 select-none overflow-hidden">
      {/* Sync notification */}
      {syncNotification && (
        <div className="fixed top-4 right-4 bg-blue-600 text-white px-4 py-2 rounded-lg shadow-lg z-50 animate-fade-in">
          {syncNotification}
        </div>
      )}

      {/* Left Panel - Project Selector */}
      <div className="w-72 bg-white border-r border-gray-200 flex flex-col">
        {/* User profile */}
        {user && (
          <div className="flex items-center gap-3 px-4 pt-4 pb-3 border-b border-gray-200">
            {user.picture ? (
              <img
                src={user.picture}
                alt={user.name || "User"}
                className="w-8 h-8 rounded-full"
                referrerPolicy="no-referrer"
              />
            ) : (
              <div className="w-8 h-8 rounded-full bg-purple-200 flex items-center justify-center text-purple-700 text-sm font-medium">
                {(user.name || user.email)[0].toUpperCase()}
              </div>
            )}
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-gray-800 truncate">
                {user.name || user.email}
              </div>
              <div className="text-xs text-gray-400 truncate">{user.email}</div>
            </div>
            <button
              onClick={logout}
              className="text-gray-400 hover:text-red-500 transition-colors shrink-0"
              title="Logout"
            >
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                />
              </svg>
            </button>
          </div>
        )}

        {/* Today's total */}
        <div className="px-4 pt-4 pb-3 border-b border-gray-200">
          <div className="flex items-center justify-between mb-1">
            <div className="text-xs uppercase tracking-wider text-gray-400">
              Today's Total
            </div>
            {overtime > 0 && (
              <div className="flex items-center gap-1 text-xs font-medium text-green-600">
                <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-8.707l-3-3a1 1 0 00-1.414 0l-3 3a1 1 0 001.414 1.414L9 9.414V13a1 1 0 102 0V9.414l1.293 1.293a1 1 0 001.414-1.414z" clipRule="evenodd" />
                </svg>
                +{formatHms(overtime)}
              </div>
            )}
          </div>
          <div className="flex items-baseline gap-2">
            <div className={`text-2xl font-mono font-semibold ${overtime > 0 ? 'text-green-600' : 'text-black'}`}>
              {formatHms(totalDaySeconds)}
            </div>
            <div className="text-xs text-gray-400">
              / 8:00:00
            </div>
          </div>
          <div className="mt-2 h-2 bg-gray-200 rounded-full overflow-hidden relative">
            {/* Base progress bar (up to 8 hours) */}
            <div
              className={`h-full rounded-full transition-all duration-300 ${
                overtime > 0 ? "bg-green-500" : "bg-purple-600"
              }`}
              style={{
                width: `${Math.min((totalDaySeconds / EIGHT_HOURS) * 100, 100)}%`,
              }}
            />
            {/* Overtime indicator - pulsing glow effect */}
            {overtime > 0 && (
              <div className="absolute inset-0 bg-green-400/30 animate-pulse rounded-full" />
            )}
          </div>
          {/* Progress milestones */}
          <div className="flex justify-between mt-1 text-[10px] text-gray-300">
            <span>0h</span>
            <span>4h</span>
            <span>8h</span>
          </div>
        </div>

        {/* Project search */}
        <div className="px-4 py-3">
          <div className="relative">
            <svg
              className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            <input
              type="text"
              placeholder="Search Projects"
              value={projectSearch}
              onChange={(e) => setProjectSearch(e.target.value)}
              className="w-full pl-9 pr-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-purple-500 focus:border-purple-500"
            />
          </div>
        </div>

        {/* Project list */}
        <div className="flex-1 overflow-y-auto">
          {filteredProjects.map((project) => {
            const isSelected = project.id === selectedProjectId;
            return (
              <div
                key={project.id}
                onClick={() => {
                  setSelectedProjectId(project.id);
                  setTaskSearch("");
                }}
                className={`flex items-center justify-between px-4 py-3 cursor-pointer transition-colors ${
                  isSelected
                    ? "bg-purple-900 text-white"
                    : "hover:bg-gray-50"
                }`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div
                    className={`w-2 h-2 rounded-full shrink-0 ${
                      project.color ? "" : isSelected ? "bg-purple-300" : "bg-purple-500"
                    }`}
                    style={project.color ? { backgroundColor: project.color } : undefined}
                  />
                  <span className={`text-sm truncate ${isSelected ? "text-white" : "text-gray-800"}`}>
                    {project.name}
                  </span>
                </div>
                <span
                  className={`text-xs px-2 py-0.5 rounded-full shrink-0 ${
                    isSelected
                      ? "bg-purple-700 text-purple-200"
                      : "bg-gray-100 text-gray-500"
                  }`}
                >
                  {project.task_count}
                </span>
              </div>
            );
          })}
          {filteredProjects.length === 0 && (
            <div className="px-4 py-8 text-center text-sm text-gray-400">
              No projects found
            </div>
          )}
        </div>
      </div>

      {/* Right Panel - Task List */}
      <div className="flex-1 flex flex-col bg-gray-50">
        {/* Header with project name */}
        <div className="px-4 py-3 bg-white border-b border-gray-200">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-gray-800">
              {selectedProject?.name || "All Tasks"}
            </h2>
            <button
              onClick={async () => {
                await refreshProjects();
                await refreshTasks();
              }}
              className="flex items-center gap-1 text-xs text-gray-500 hover:text-purple-600 transition-colors"
              title="Refresh"
            >
              <svg
                className="w-3.5 h-3.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                />
              </svg>
              Refresh
            </button>
          </div>
          {/* Task search */}
          <div className="relative">
            <svg
              className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
              />
            </svg>
            <input
              type="text"
              placeholder="Search task"
              value={taskSearch}
              onChange={(e) => setTaskSearch(e.target.value)}
              className="w-full pl-9 pr-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-purple-500 focus:border-purple-500"
            />
          </div>
        </div>

        {/* Task list */}
        <div className="flex-1 overflow-y-auto">
          {filteredTasks.map((task) => {
            const isActive = task.running;
            const isExceeded = task.elapsed_secs > task.budget_secs;

            return (
              <div
                key={task.id}
                onClick={() => setSelectedTaskId(task.id)}
                className={`flex items-center justify-between px-4 py-3 border-b border-gray-200 cursor-pointer transition-colors ${
                  isActive
                    ? "bg-purple-900 text-white"
                    : selectedTaskId === task.id
                      ? "bg-purple-50"
                      : "bg-white hover:bg-gray-50"
                }`}
              >
                <div className="flex-1 min-w-0 mr-3">
                  <span
                    className={`text-sm truncate block ${isActive ? "text-white" : "text-gray-800"}`}
                  >
                    {task.name}
                  </span>
                </div>

                <div className="flex items-center gap-3 shrink-0">
                  {isExceeded && (
                    <span
                      className={`text-[10px] px-2 py-0.5 rounded-full font-semibold tracking-wide ${
                        isActive
                          ? "bg-red-500/20 text-red-200"
                          : "bg-red-100 text-red-700"
                      }`}
                    >
                      Time Exceeded
                    </span>
                  )}
                  <span
                    className={`text-sm font-mono ${isActive ? "text-purple-200" : "text-gray-500"}`}
                  >
                    {formatHms(task.elapsed_secs)} /{" "}
                    {formatHms(task.budget_secs)}
                  </span>

                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleTimer(task.id);
                    }}
                    className={`w-8 h-8 flex items-center justify-center rounded-full transition-colors ${
                      isActive
                        ? "bg-white/20 hover:bg-white/30 text-white"
                        : "bg-gray-100 hover:bg-gray-200 text-gray-600"
                    }`}
                  >
                    {isActive ? (
                      <svg
                        className="w-4 h-4"
                        fill="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <rect x="6" y="4" width="4" height="16" rx="1" />
                        <rect x="14" y="4" width="4" height="16" rx="1" />
                      </svg>
                    ) : (
                      <svg
                        className="w-4 h-4"
                        fill="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path d="M8 5v14l11-7z" />
                      </svg>
                    )}
                  </button>
                </div>
              </div>
            );
          })}
          {filteredTasks.length === 0 && (
            <div className="px-4 py-8 text-center text-sm text-gray-400">
              {selectedProjectId ? "No tasks in this project" : "No tasks available"}
            </div>
          )}
        </div>

        {/* Task detail footer */}
        {selectedTask && (
          <div className="border-t border-gray-200 bg-white px-5 py-4">
            <h3 className="text-sm font-semibold text-gray-800">
              {selectedTask.name}
            </h3>
            <div className="flex items-center gap-2 mt-1">
              {selectedTask.elapsed_secs > selectedTask.budget_secs && (
                <span className="text-[10px] px-2 py-0.5 rounded-full font-semibold tracking-wide bg-red-100 text-red-700">
                  Time Exceeded
                </span>
              )}
              <p className="text-xs text-gray-400">
                Elapsed: {formatHms(selectedTask.elapsed_secs)} / Budget:{" "}
                {formatHms(selectedTask.budget_secs)}
              </p>
            </div>
          </div>
        )}

        {/* Status bar */}
        <div className="flex items-center justify-between px-4 py-2 bg-gray-100 border-t border-gray-200 text-xs text-gray-400">
          <span>
            Last updated on {new Date().toLocaleDateString("en-GB")},{" "}
            {new Date().toLocaleTimeString("en-GB")}
          </span>
          <span>v.1.0.0</span>
        </div>
      </div>

      {/* Idle Time Alert Modal */}
      {idleEvent && (
        <div className="fixed inset-0 bg-black/30 flex items-start justify-center pt-24 z-50">
          <div className="w-full max-w-md overflow-hidden rounded-lg border border-gray-200 bg-white shadow-2xl">
            <div className="px-4 py-2.5 border-b border-gray-200 bg-gray-50 text-center text-[12px] font-semibold tracking-wide text-gray-700">
              Idle Time Alert
            </div>
            <div className="p-5 text-[12px] text-gray-700 space-y-4">
              <div className="leading-5">
                You have been idle for{" "}
                <span className="font-semibold text-gray-900">
                  {formatIdleDuration(idleEvent.idle_duration_secs)}
                </span>
                .
              </div>
              <div className="space-y-2 rounded-md border border-gray-200 bg-gray-50 px-3 py-3 text-[11px] text-gray-500">
                <div className="flex items-start justify-between gap-3">
                  Project:{" "}
                  <span className="text-right font-medium text-gray-800">
                    {idleEvent.task_name}
                  </span>
                </div>
                <div className="flex items-center justify-between gap-3">
                  <span>Assign time to:</span>
                  <select
                    value={idleTaskId ?? ""}
                    onChange={(e) => setIdleTaskId(Number(e.target.value))}
                    disabled={idleEvent.tracking_active}
                    className="min-w-[190px] rounded-sm border border-gray-300 bg-white px-2 py-1.5 text-[11px] text-gray-700 disabled:bg-gray-100"
                  >
                    {tasks.map((task) => (
                      <option key={task.id} value={task.id}>
                        {task.name}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              {idleEvent.tracking_active && (
                <div className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-[11px] text-amber-700">
                  Idle time is still increasing. Actions unlock when you are
                  active again.
                </div>
              )}
              {!idleEvent.tracking_active && (
                <div className="text-[11px] text-gray-500">
                  Keep this time on the selected task or discard it.
                </div>
              )}
              <div className="flex items-center justify-end gap-2 pt-1">
                <button
                  onClick={handleDiscardIdleTime}
                  disabled={idleEvent.tracking_active}
                  className="rounded-sm border border-gray-300 bg-white px-3 py-1.5 text-[12px] text-gray-700 hover:bg-gray-50 disabled:bg-gray-100 disabled:text-gray-400"
                >
                  Discard idle time
                </button>
                <button
                  onClick={handleKeepIdleTime}
                  disabled={idleEvent.tracking_active || !idleTaskId}
                  className="rounded-sm border border-blue-600 bg-blue-600 px-3 py-1.5 text-[12px] text-white hover:bg-blue-700 disabled:border-blue-300 disabled:bg-blue-300"
                >
                  Keep idle time
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Quit Confirmation Modal */}
      {quitConfirmOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-red-100 rounded-full flex items-center justify-center">
                <svg
                  className="w-5 h-5 text-red-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01M5.07 19h13.86c1.54 0 2.5-1.67 1.73-3L13.73 4c-.77-1.33-2.69-1.33-3.46 0L3.34 16c-.77 1.33.19 3 1.73 3z"
                  />
                </svg>
              </div>
              <div>
                <h2 className="text-lg font-semibold text-gray-800">
                  Quit the app?
                </h2>
                <p className="text-xs text-gray-400">
                  {quitHasRunning
                    ? "A timer is running. Stopping it will sync time to the server."
                    : "Your data is up to date. You can quit safely."}
                </p>
              </div>
            </div>

            {quitError && (
              <div className="bg-red-50 text-red-700 text-xs rounded-md px-3 py-2 mb-3">
                {quitError}
              </div>
            )}

            <div className="flex gap-3">
              <button
                onClick={() => setQuitConfirmOpen(false)}
                className="flex-1 bg-gray-200 text-gray-800 rounded-md py-2.5 text-sm font-medium hover:bg-gray-300 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmQuit}
                className="flex-1 bg-red-600 text-white rounded-md py-2.5 text-sm font-medium hover:bg-red-700 transition-colors"
              >
                {quitHasRunning ? "Stop and Quit" : "Quit"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Crash Recovery / Time Discarded Modal */}
      {crashRecoveryOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-10 h-10 bg-amber-100 rounded-full flex items-center justify-center">
                <svg
                  className="w-5 h-5 text-amber-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                  />
                </svg>
              </div>
              <div>
                <h2 className="text-lg font-semibold text-gray-800">
                  Time Verification
                </h2>
                <p className="text-xs text-gray-400">
                  Timestamp mismatch detected. Unwanted time has been discarded.
                </p>
              </div>
            </div>

            <div className="bg-amber-50 border border-amber-200 rounded-md px-4 py-3 mb-4">
              <p className="text-sm text-amber-800">
                The recorded time has been verified against the server.
                Any discrepancy between local and server timestamps has been corrected
                to ensure accurate time tracking.
                {crashRecoveredTaskId && (
                  <span className="block mt-2 font-medium">
                    Task ID: {crashRecoveredTaskId}
                  </span>
                )}
              </p>
            </div>

            <button
              onClick={() => {
                setCrashRecoveryOpen(false);
                setCrashRecoveredTaskId(null);
                refreshTasks();
              }}
              className="w-full bg-amber-600 text-white rounded-md py-2.5 text-sm font-medium hover:bg-amber-700 transition-colors"
            >
              Got it
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
