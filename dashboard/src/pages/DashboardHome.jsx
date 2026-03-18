import { useState, useEffect } from 'react';
import { Users, ListTodo, Activity, Clock } from 'lucide-react';
import { dashboardApi } from '../api';
import StatCard from '../components/StatCard';
import { formatHours, formatDuration } from '../utils/format';

export default function DashboardHome() {
  const [stats, setStats] = useState(null);
  const [users, setUsers] = useState([]);
  const [loading, setLoading] = useState(true);

  const fetchData = async () => {
    try {
      const [statsData, usersData] = await Promise.all([
        dashboardApi.getStats(),
        dashboardApi.getUsersStatus(),
      ]);
      setStats(statsData);
      setUsers(usersData);
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

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-gray-800">Dashboard</h1>
        <p className="text-gray-500 mt-1">Real-time employee tracking and statistics</p>
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

      <div className="bg-white rounded-2xl shadow-sm border border-gray-100 overflow-hidden">
        <div className="p-6 border-b border-gray-100">
          <h2 className="text-lg font-semibold text-gray-800">Employee Status</h2>
        </div>
        
        {users.length === 0 ? (
          <div className="p-12 text-center text-gray-500">
            No employees found
          </div>
        ) : (
          <div className="divide-y divide-gray-100">
            {users.map(user => (
              <div key={user.user_id} className="p-4 flex items-center justify-between hover:bg-gray-50 transition-colors">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center overflow-hidden">
                    {user.user_picture ? (
                      <img src={user.user_picture} alt={user.user_name} className="w-full h-full object-cover" />
                    ) : (
                      <span className="text-white font-medium">
                        {user.user_name?.charAt(0).toUpperCase() || '?'}
                      </span>
                    )}
                  </div>
                  <div>
                    <p className="font-medium text-gray-800">{user.user_name}</p>
                    {user.running ? (
                      <p className="text-sm text-green-600">Working: {user.task_name}</p>
                    ) : (
                      <p className="text-sm text-gray-400">Idle</p>
                    )}
                  </div>
                </div>
                <div className="text-right">
                  {user.running ? (
                    <>
                      <span className="inline-flex items-center gap-1.5 text-sm text-green-600 font-medium">
                        <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
                        Working
                      </span>
                      <p className="text-lg font-mono text-gray-800 mt-1">
                        {formatDuration(user.elapsed_seconds)}
                      </p>
                    </>
                  ) : (
                    <span className="text-sm text-gray-400">Offline</span>
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
