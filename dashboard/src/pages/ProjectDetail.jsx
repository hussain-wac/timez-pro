import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Users, ListTodo, Clock, Settings, BarChart3 } from 'lucide-react';
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

  const colorClasses = {
    blue: 'from-blue-400 to-blue-600',
    purple: 'from-purple-400 to-purple-600',
    green: 'from-green-400 to-green-600',
    orange: 'from-orange-400 to-orange-600',
    pink: 'from-pink-400 to-pink-600',
    indigo: 'from-indigo-400 to-indigo-600',
    red: 'from-red-400 to-red-600',
    teal: 'from-teal-400 to-teal-600',
  };

  const gradientClass = colorClasses[project.color] || colorClasses.blue;
  const allTasks = [...tasks.todo, ...tasks.in_progress, ...tasks.review, ...tasks.done];
  const activeTasks = allTasks.filter(t => t.status !== 'done').length;
  const totalHours = allTasks.reduce((sum, task) => sum + (task.total_tracked_seconds || 0), 0);

  return (
    <div className="space-y-6">
      <div>
        <button
          onClick={() => navigate('/projects')}
          className="flex items-center gap-2 text-gray-500 hover:text-gray-700 mb-4 transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
          <span className="text-sm">Back to Projects</span>
        </button>

        <div className="flex items-start gap-4">
          <div className={`w-16 h-16 rounded-2xl bg-gradient-to-br ${gradientClass} flex items-center justify-center shadow-sm flex-shrink-0`}>
            <span className="text-white text-2xl font-bold">
              {project.name?.charAt(0).toUpperCase() || 'P'}
            </span>
          </div>
          <div className="flex-1">
            <h1 className="text-2xl font-bold text-gray-800">{project.name}</h1>
            {project.description && (
              <p className="text-gray-500 mt-1">{project.description}</p>
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
              className={`flex items-center gap-2 px-4 py-3 border-b-2 transition-colors ${
                activeTab === tab.id
                  ? 'border-blue-600 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700'
              }`}
            >
              <tab.icon className="w-4 h-4" />
              <span className="font-medium">{tab.label}</span>
            </button>
          ))}
        </nav>
      </div>

      {activeTab === 'overview' && (
        <div className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <StatCard
              title="Total Hours"
              value={formatHours(totalHours)}
              icon={Clock}
              color="text-orange-600"
              iconBg="bg-orange-50"
            />
            <StatCard
              title="Active Tasks"
              value={activeTasks}
              icon={ListTodo}
              color="text-blue-600"
              iconBg="bg-blue-50"
            />
            <StatCard
              title="Team Members"
              value={project.member_count || 0}
              icon={Users}
              color="text-purple-600"
              iconBg="bg-purple-50"
            />
          </div>

          <div className="bg-white rounded-2xl shadow-sm border border-gray-100 p-6">
            <h3 className="text-lg font-semibold text-gray-800 mb-4">Task Overview</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {statusColumns.map(col => (
                <div key={col.key} className="text-center">
                  <p className="text-sm text-gray-500 mb-1">{col.title}</p>
                  <p className="text-3xl font-bold text-gray-800">{tasks[col.key].length}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {activeTab === 'tasks' && (
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
      )}

      {activeTab === 'members' && (
        <div className="bg-white rounded-2xl shadow-sm border border-gray-100 p-6">
          <MemberManager projectId={id} />
        </div>
      )}
    </div>
  );
}
