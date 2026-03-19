import { useState, useEffect, useCallback, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Users, ListTodo, Clock, Settings, BarChart3, Plus, X } from 'lucide-react';
import { dashboardApi } from '../api';
import StatCard from '../components/StatCard';
import MemberManager from '../components/MemberManager';
import KanbanColumn from '../components/KanbanColumn';
import { formatHours } from '../utils/format';

const TABS = [
  { id: 'overview', label: 'Overview', icon: BarChart3 },
  { id: 'tasks', label: 'Tasks', icon: ListTodo },
  { id: 'members', label: 'Members', icon: Users },
];

const statusColumns = [
  { key: 'todo', title: 'To Do', countColor: 'bg-gray-100 text-gray-600' },
  { key: 'in_progress', title: 'In Progress', countColor: 'bg-blue-100 text-blue-600' },
  { key: 'review', title: 'Review', countColor: 'bg-yellow-100 text-yellow-600' },
  { key: 'done', title: 'Done', countColor: 'bg-green-100 text-green-600' },
];

export default function ProjectDetail() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [project, setProject] = useState(null);
  const [tasks, setTasks] = useState({ todo: [], in_progress: [], review: [], done: [] });
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState('overview');
  const [showCreateTask, setShowCreateTask] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState(null);
  const [projectMembers, setProjectMembers] = useState([]);
  const [selectedAssignees, setSelectedAssignees] = useState([]);
  const [loadingMembers, setLoadingMembers] = useState(false);
  const [newTask, setNewTask] = useState({
    name: '',
    description: '',
    max_hours: 8,
    priority: 'medium',
    status: 'todo'
  });

  // Track mount state to prevent state updates after unmount
  const isMountedRef = useRef(true);
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const fetchProject = useCallback(async () => {
    try {
      const data = await dashboardApi.getProject(id);
      if (isMountedRef.current) {
        setProject(data);
        setError(null);
      }
    } catch (err) {
      console.error('Failed to fetch project:', err);
      if (isMountedRef.current) {
        setError('Failed to load project');
      }
    }
  }, [id]);

  const fetchTasks = useCallback(async () => {
    try {
      const data = await dashboardApi.getProjectTasks(id);
      if (isMountedRef.current) {
        // Group tasks by status
        const grouped = { todo: [], in_progress: [], review: [], done: [] };
        data.forEach(task => {
          const status = task.status || 'todo';
          if (grouped[status]) {
            grouped[status].push(task);
          }
        });
        setTasks(grouped);
      }
    } catch (err) {
      console.error('Failed to fetch tasks:', err);
    }
  }, [id]);

  useEffect(() => {
    let cancelled = false;
    const loadData = async () => {
      setLoading(true);
      await Promise.all([fetchProject(), fetchTasks()]);
      if (!cancelled) {
        setLoading(false);
      }
    };
    loadData();
    return () => {
      cancelled = true;
    };
  }, [id, fetchProject, fetchTasks]);

  // Fetch project members when create task modal opens
  const fetchProjectMembers = useCallback(async () => {
    setLoadingMembers(true);
    try {
      const members = await dashboardApi.getProjectMembers(id);
      if (isMountedRef.current) {
        setProjectMembers(members);
        // Select all members by default
        setSelectedAssignees(members.map(m => m.user_id));
      }
    } catch (err) {
      console.error('Failed to fetch project members:', err);
    } finally {
      if (isMountedRef.current) {
        setLoadingMembers(false);
      }
    }
  }, [id]);

  const handleOpenCreateModal = useCallback(() => {
    setShowCreateTask(true);
    fetchProjectMembers();
  }, [fetchProjectMembers]);

  const handleCloseCreateModal = useCallback(() => {
    setShowCreateTask(false);
    setError(null);
    setSelectedAssignees([]);
    setNewTask({ name: '', description: '', max_hours: 8, priority: 'medium', status: 'todo' });
  }, []);

  const toggleAssignee = useCallback((userId) => {
    setSelectedAssignees(prev =>
      prev.includes(userId)
        ? prev.filter(id => id !== userId)
        : [...prev, userId]
    );
  }, []);

  const handleStatusChange = useCallback(async (taskId, newStatus) => {
    // Optimistic update for smoother UI
    setTasks(prev => {
      const newTasks = { ...prev };
      let movedTask = null;

      // Find and remove the task from its current column
      for (const status of Object.keys(newTasks)) {
        const index = newTasks[status].findIndex(t => t.id === taskId);
        if (index !== -1) {
          movedTask = { ...newTasks[status][index], status: newStatus };
          newTasks[status] = [...newTasks[status].slice(0, index), ...newTasks[status].slice(index + 1)];
          break;
        }
      }

      // Add to new column
      if (movedTask && newTasks[newStatus]) {
        newTasks[newStatus] = [...newTasks[newStatus], movedTask];
      }

      return newTasks;
    });

    try {
      await dashboardApi.updateTaskStatus(taskId, newStatus);
      // Refresh to get server state
      if (isMountedRef.current) {
        await fetchTasks();
      }
    } catch (err) {
      console.error('Failed to update task status:', err);
      // Revert on error by refetching
      if (isMountedRef.current) {
        await fetchTasks();
      }
    }
  }, [fetchTasks]);

  const handleCreateTask = useCallback(async (e) => {
    e.preventDefault();
    if (!newTask.name.trim()) return;

    if (selectedAssignees.length === 0) {
      setError('Please select at least one assignee.');
      return;
    }

    setCreating(true);
    setError(null);
    try {
      const taskData = {
        ...newTask,
        assignee_ids: selectedAssignees
      };
      await dashboardApi.createProjectTask(id, taskData);
      if (isMountedRef.current) {
        await fetchTasks();
        handleCloseCreateModal();
      }
    } catch (err) {
      console.error('Failed to create task:', err);
      if (isMountedRef.current) {
        setError('Failed to create task. Please try again.');
      }
    } finally {
      if (isMountedRef.current) {
        setCreating(false);
      }
    }
  }, [id, newTask, selectedAssignees, fetchTasks, handleCloseCreateModal]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (!project) {
    return (
      <div className="text-center py-12">
        <p className="text-gray-500">Project not found</p>
        <button
          onClick={() => navigate('/projects')}
          className="mt-4 text-blue-600 hover:underline"
        >
          Back to Projects
        </button>
      </div>
    );
  }

  const allTasks = [...tasks.todo, ...tasks.in_progress, ...tasks.review, ...tasks.done];
  const activeTasks = allTasks.filter(t => t.status !== 'done').length;
  const totalHours = allTasks.reduce((sum, task) => sum + (task.total_tracked_seconds || 0), 0);

  const colorClasses = {
    blue: 'bg-blue-500',
    purple: 'bg-purple-500',
    green: 'bg-green-500',
    orange: 'bg-orange-500',
    pink: 'bg-pink-500',
    indigo: 'bg-indigo-500',
    red: 'bg-red-500',
    teal: 'bg-teal-500',
  };

  const colorClass = colorClasses[project.color] || colorClasses.blue;

  return (
    <div className="space-y-6">
      <div>
        <button
          onClick={() => navigate('/projects')}
          className="flex items-center gap-2 text-gray-600 hover:text-gray-900 mb-4 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          <span className="text-sm">Back to Projects</span>
        </button>

        <div className="flex items-start gap-4">
          <div className={`w-12 h-12 rounded-md ${colorClass} flex items-center justify-center flex-shrink-0`}>
            <span className="text-white text-lg font-semibold">
              {project.name?.charAt(0).toUpperCase() || 'P'}
            </span>
          </div>
          <div className="flex-1">
            <h1 className="text-2xl font-semibold text-gray-900">{project.name}</h1>
            {project.description && (
              <p className="text-gray-600 mt-1 text-sm">{project.description}</p>
            )}
          </div>
        </div>
      </div>

      <div className="border-b border-gray-200">
        <nav className="flex gap-6">
          {TABS.map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-4 py-3 border-b-2 transition-colors text-sm ${
                activeTab === tab.id
                  ? 'border-blue-600 text-blue-600 font-medium'
                  : 'border-transparent text-gray-600 hover:text-gray-900'
              }`}
            >
              <tab.icon className="w-4 h-4" />
              <span>{tab.label}</span>
            </button>
          ))}
        </nav>
      </div>

      {activeTab === 'overview' && (
        <div className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="bg-white border border-gray-200 rounded-md p-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-600 mb-1">Total Hours</p>
                  <p className="text-2xl font-semibold text-gray-900">{formatHours(totalHours)}</p>
                </div>
                <div className="w-10 h-10 rounded-md bg-orange-50 flex items-center justify-center">
                  <Clock className="w-5 h-5 text-orange-600" />
                </div>
              </div>
            </div>
            <div className="bg-white border border-gray-200 rounded-md p-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-600 mb-1">Active Tasks</p>
                  <p className="text-2xl font-semibold text-gray-900">{activeTasks}</p>
                </div>
                <div className="w-10 h-10 rounded-md bg-blue-50 flex items-center justify-center">
                  <ListTodo className="w-5 h-5 text-blue-600" />
                </div>
              </div>
            </div>
            <div className="bg-white border border-gray-200 rounded-md p-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-600 mb-1">Team Members</p>
                  <p className="text-2xl font-semibold text-gray-900">{project.member_count || 0}</p>
                </div>
                <div className="w-10 h-10 rounded-md bg-purple-50 flex items-center justify-center">
                  <Users className="w-5 h-5 text-purple-600" />
                </div>
              </div>
            </div>
          </div>

          <div className="bg-white rounded-md border border-gray-200 p-6">
            <h3 className="text-base font-medium text-gray-900 mb-4">Task Distribution</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {statusColumns.map(col => (
                <div key={col.key} className="bg-gray-50 rounded-md p-4 text-center">
                  <p className="text-xs text-gray-600 mb-2 uppercase tracking-wide">{col.title}</p>
                  <p className="text-3xl font-semibold text-gray-900">{tasks[col.key].length}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {activeTab === 'tasks' && (
        <div className="space-y-4">
          <div className="flex justify-end">
            <button
              onClick={handleOpenCreateModal}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors text-sm font-medium"
            >
              <Plus className="w-4 h-4" />
              Create Task
            </button>
          </div>
          <div className="flex gap-4 overflow-x-auto pb-4">
            {statusColumns.map(col => (
              <KanbanColumn
                key={col.key}
                title={col.title}
                currentStatus={col.key}
                tasks={tasks[col.key] || []}
                onStatusChange={handleStatusChange}
                countColor={col.countColor}
              />
            ))}
          </div>
        </div>
      )}

      {/* Create Task Modal */}
      {showCreateTask && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-md shadow-lg w-full max-w-md mx-4 max-h-[90vh] overflow-y-auto">
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200 sticky top-0 bg-white">
              <h3 className="text-base font-medium text-gray-900">Create New Task</h3>
              <button onClick={handleCloseCreateModal} className="text-gray-400 hover:text-gray-600">
                <X className="w-5 h-5" />
              </button>
            </div>
            <form onSubmit={handleCreateTask} className="p-4 space-y-4">
              {error && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-md text-sm text-red-700">
                  {error}
                </div>
              )}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Task Name *</label>
                <input
                  type="text"
                  value={newTask.name}
                  onChange={(e) => setNewTask({ ...newTask, name: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                  placeholder="Enter task name"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Description</label>
                <textarea
                  value={newTask.description}
                  onChange={(e) => setNewTask({ ...newTask, description: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                  placeholder="Enter task description"
                  rows={3}
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Max Hours</label>
                  <input
                    type="number"
                    value={newTask.max_hours}
                    onChange={(e) => setNewTask({ ...newTask, max_hours: parseInt(e.target.value) || 1 })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                    min="1"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Priority</label>
                  <select
                    value={newTask.priority}
                    onChange={(e) => setNewTask({ ...newTask, priority: e.target.value })}
                    className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                  >
                    <option value="low">Low</option>
                    <option value="medium">Medium</option>
                    <option value="high">High</option>
                    <option value="urgent">Urgent</option>
                  </select>
                </div>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Initial Status</label>
                <select
                  value={newTask.status}
                  onChange={(e) => setNewTask({ ...newTask, status: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="todo">To Do</option>
                  <option value="in_progress">In Progress</option>
                  <option value="review">Review</option>
                  <option value="done">Done</option>
                </select>
              </div>

              {/* Assignees Selection */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Assign To *
                  <span className="text-gray-400 font-normal ml-1">
                    ({selectedAssignees.length} selected)
                  </span>
                </label>
                {loadingMembers ? (
                  <div className="flex items-center justify-center py-4">
                    <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600"></div>
                    <span className="ml-2 text-sm text-gray-500">Loading members...</span>
                  </div>
                ) : projectMembers.length === 0 ? (
                  <div className="p-3 bg-yellow-50 border border-yellow-200 rounded-md text-sm text-yellow-700">
                    No members in this project. Add members first.
                  </div>
                ) : (
                  <div className="border border-gray-300 rounded-md max-h-40 overflow-y-auto">
                    {projectMembers.map(member => (
                      <label
                        key={member.user_id}
                        className="flex items-center gap-3 px-3 py-2 hover:bg-gray-50 cursor-pointer border-b border-gray-100 last:border-b-0"
                      >
                        <input
                          type="checkbox"
                          checked={selectedAssignees.includes(member.user_id)}
                          onChange={() => toggleAssignee(member.user_id)}
                          className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                        />
                        <div className="flex items-center gap-2 flex-1 min-w-0">
                          {member.user?.picture ? (
                            <img
                              src={member.user.picture}
                              alt=""
                              className="w-6 h-6 rounded-full"
                            />
                          ) : (
                            <div className="w-6 h-6 rounded-full bg-gray-200 flex items-center justify-center">
                              <span className="text-xs text-gray-600">
                                {(member.user?.name || member.user?.email || '?').charAt(0).toUpperCase()}
                              </span>
                            </div>
                          )}
                          <div className="flex-1 min-w-0">
                            <p className="text-sm text-gray-900 truncate">
                              {member.user?.name || member.user?.email}
                            </p>
                            {member.user?.name && (
                              <p className="text-xs text-gray-500 truncate">{member.user.email}</p>
                            )}
                          </div>
                          {member.role === 'lead' && (
                            <span className="text-xs bg-blue-100 text-blue-700 px-1.5 py-0.5 rounded">Lead</span>
                          )}
                        </div>
                      </label>
                    ))}
                  </div>
                )}
                <div className="flex gap-2 mt-2">
                  <button
                    type="button"
                    onClick={() => setSelectedAssignees(projectMembers.map(m => m.user_id))}
                    className="text-xs text-blue-600 hover:text-blue-700"
                  >
                    Select All
                  </button>
                  <span className="text-gray-300">|</span>
                  <button
                    type="button"
                    onClick={() => setSelectedAssignees([])}
                    className="text-xs text-blue-600 hover:text-blue-700"
                  >
                    Clear All
                  </button>
                </div>
              </div>

              <div className="flex gap-3 pt-2">
                <button
                  type="button"
                  onClick={handleCloseCreateModal}
                  className="flex-1 px-4 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 hover:bg-gray-50"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={creating || !newTask.name.trim() || selectedAssignees.length === 0}
                  className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md text-sm font-medium hover:bg-blue-700 disabled:opacity-50"
                >
                  {creating ? 'Creating...' : 'Create Task'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {activeTab === 'members' && (
        <div className="bg-white rounded-md border border-gray-200 p-6">
          <MemberManager projectId={id} />
        </div>
      )}
    </div>
  );
}
