import { useState, useEffect } from 'react';
import { Users, ListTodo, Activity, Clock, FolderKanban, ArrowRight } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { dashboardApi } from '../api';
import StatCard from '../components/StatCard';
import ProjectCard from '../components/ProjectCard';
import { formatHours, formatDuration } from '../utils/format';

export default function DashboardHome() {
  const navigate = useNavigate();
  const [stats, setStats] = useState(null);
  const [users, setUsers] = useState([]);
  const [projects, setProjects] = useState([]);
  const [loading, setLoading] = useState(true);

  const fetchData = async () => {
    try {
      const [statsData, usersData, projectsData] = await Promise.all([
        dashboardApi.getStats(),
        dashboardApi.getUsersStatus(),
        dashboardApi.getProjects(),
      ]);
      setStats(statsData);
      setUsers(usersData);
      setProjects(projectsData);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  const workingUsers = users.filter(u => u.running);
  const recentProjects = projects.slice(0, 4);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Dashboard</h1>
        <p className="text-gray-600 mt-1 text-sm">Real-time employee tracking and statistics</p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="Total Employees"
          value={stats?.total_users || 0}
          icon={Users}
          color="text-blue-600"
          iconBg="bg-blue-50"
        />
        <StatCard
          title="Total Tasks"
          value={stats?.total_tasks || 0}
          icon={ListTodo}
          color="text-purple-600"
          iconBg="bg-purple-50"
        />
        <StatCard
          title="Currently Working"
          value={stats?.currently_working || 0}
          icon={Activity}
          color="text-green-600"
          iconBg="bg-green-50"
        />
        <StatCard
          title="Today's Total"
          value={formatHours(stats?.today_total_seconds || 0)}
          icon={Clock}
          color="text-orange-600"
          iconBg="bg-orange-50"
        />
      </div>

      {projects.length > 0 && (
        <div className="bg-white rounded-md border border-gray-200 p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-base font-medium text-gray-900 flex items-center gap-2">
              <FolderKanban className="w-5 h-5 text-blue-600" />
              Recent Projects
            </h2>
            <button
              onClick={() => navigate('/projects')}
              className="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-700 font-medium transition-colors"
            >
              View All
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {recentProjects.map(project => (
              <ProjectCard key={project.id} project={project} />
            ))}
          </div>
        </div>
      )}

      <div className="bg-white rounded-md border border-gray-200 overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-200">
          <h2 className="text-base font-medium text-gray-900">Employee Status</h2>
        </div>

        {users.length === 0 ? (
          <div className="p-12 text-center text-gray-600 text-sm">
            No employees found
          </div>
        ) : (
          <div className="divide-y divide-gray-200">
            {users.map(user => (
              <div key={user.user_id} className="px-6 py-4 flex items-center justify-between hover:bg-gray-50 transition-colors">
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 rounded-md bg-blue-500 flex items-center justify-center overflow-hidden flex-shrink-0">
                    {user.user_picture ? (
                      <img src={user.user_picture} alt={user.user_name} className="w-full h-full object-cover" />
                    ) : (
                      <span className="text-white text-sm font-semibold">
                        {user.user_name?.charAt(0).toUpperCase() || '?'}
                      </span>
                    )}
                  </div>
                  <div>
                    <p className="font-medium text-gray-900 text-sm">{user.user_name}</p>
                    {user.running ? (
                      <p className="text-xs text-green-600">Working: {user.task_name}</p>
                    ) : (
                      <p className="text-xs text-gray-500">Idle</p>
                    )}
                  </div>
                </div>
                <div className="text-right">
                  {user.running ? (
                    <>
                      <span className="inline-flex items-center gap-1.5 text-xs text-green-600 font-medium">
                        <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
                        Working
                      </span>
                      <p className="text-base font-mono text-gray-900 mt-1">
                        {formatDuration(user.elapsed_seconds)}
                      </p>
                    </>
                  ) : (
                    <span className="text-xs text-gray-500">Offline</span>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
