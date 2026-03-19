import { useState, useEffect } from 'react';
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
  const [newTask, setNewTask] = useState({
    name: '',
    description: '',
    max_hours: 8,
    priority: 'medium',
    status: 'todo'
  });

  const fetchProject = async () => {
    try {
      const data = await dashboardApi.getProject(id);
      setProject(data);
    } catch (err) {
      console.error('Failed to fetch project:', err);
    }
  };

  const fetchTasks = async () => {
    try {
      const data = await dashboardApi.getProjectTasks(id);
      // Group tasks by status
      const grouped = { todo: [], in_progress: [], review: [], done: [] };
      data.forEach(task => {
        const status = task.status || 'todo';
        if (grouped[status]) {
          grouped[status].push(task);
        }
      });
      setTasks(grouped);
    } catch (err) {
      console.error('Failed to fetch tasks:', err);
    }
  };

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      await Promise.all([fetchProject(), fetchTasks()]);
      setLoading(false);
    };
    loadData();
  }, [id]);

  const handleStatusChange = async (taskId, newStatus) => {
    try {
      await dashboardApi.updateTaskStatus(taskId, newStatus);
      await fetchTasks();
    } catch (err) {
      console.error('Failed to update task status:', err);
    }
  };

  const handleCreateTask = async (e) => {
    e.preventDefault();
    if (!newTask.name.trim()) return;

    setCreating(true);
    try {
      await dashboardApi.createProjectTask(id, newTask);
      await fetchTasks();
      setShowCreateTask(false);
      setNewTask({ name: '', description: '', max_hours: 8, priority: 'medium', status: 'todo' });
    } catch (err) {
      console.error('Failed to create task:', err);
      alert('Failed to create task');
    } finally {
      setCreating(false);
    }
  };

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
              onClick={() => setShowCreateTask(true)}
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
          <div className="bg-white rounded-md shadow-lg w-full max-w-md mx-4">
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200">
              <h3 className="text-base font-medium text-gray-900">Create New Task</h3>
              <button onClick={() => setShowCreateTask(false)} className="text-gray-400 hover:text-gray-600">
                <X className="w-5 h-5" />
              </button>
            </div>
            <form onSubmit={handleCreateTask} className="p-4 space-y-4">
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
              <div className="flex gap-3 pt-2">
                <button
                  type="button"
                  onClick={() => setShowCreateTask(false)}
                  className="flex-1 px-4 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 hover:bg-gray-50"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={creating || !newTask.name.trim()}
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
