import { Routes, Route, NavLink, Navigate } from 'react-router-dom';
import { useState, useEffect } from 'react';
import Login from './Login';
import { dashboardApi } from './api';
import Layout from './components/Layout';
import DashboardHome from './pages/DashboardHome';
import Employees from './pages/Employees';
import Kanban from './pages/Kanban';

function App() {
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = localStorage.getItem('token');
    const userData = localStorage.getItem('user');
    if (token && userData) {
      const parsedUser = JSON.parse(userData);
      setUser(parsedUser);
      dashboardApi.getCurrentUser()
        .then(userInfo => {
          setUser(userInfo);
          localStorage.setItem('user', JSON.stringify(userInfo));
        })
        .catch(() => {
          localStorage.removeItem('token');
          localStorage.removeItem('user');
          setUser(null);
        })
        .finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
  }, []);

  const handleLogout = () => {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    setUser(null);
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-gray-500">Loading...</div>
      </div>
    );
  }

  if (!user) {
    return <Login onLogin={setUser} />;
  }

  if (!user.is_admin) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center p-4">
        <div className="bg-white rounded-2xl shadow-lg p-8 text-center">
          <h1 className="text-2xl font-bold text-gray-800 mb-4">Access Denied</h1>
          <p className="text-gray-500 mb-6">You don't have admin access to view this dashboard.</p>
          <button
            onClick={handleLogout}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            Logout
          </button>
        </div>
      </div>
    );
  }

  return (
    <Layout user={user} onLogout={handleLogout}>
      <Routes>
        <Route path="/" element={<DashboardHome />} />
        <Route path="/employees" element={<Employees />} />
        <Route path="/employees/:userId" element={<Kanban />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Layout>
  );
}

export default App;
