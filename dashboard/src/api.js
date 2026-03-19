const API_BASE = import.meta.env.VITE_API_URL || '';

const getAuthHeaders = () => {
  const token = localStorage.getItem('token');
  return token ? { Authorization: `Bearer ${token}` } : {};
};

export const api = {
  async get(endpoint) {
    const res = await fetch(`${API_BASE}${endpoint}`, {
      headers: {
        'Content-Type': 'application/json',
        ...getAuthHeaders(),
      },
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  },

  async post(endpoint, data) {
    const res = await fetch(`${API_BASE}${endpoint}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...getAuthHeaders(),
      },
      body: JSON.stringify(data),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  },

  async login(token) {
    console.log('API login called with token:', token ? 'present' : 'missing');
    const res = await fetch(`${API_BASE}/api/auth/google`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
    });
    const data = await res.json();
    if (!res.ok) {
      console.error('Login API error:', res.status, data);
      throw new Error(data.detail || `HTTP ${res.status}`);
    }
    return data;
  },
};

export const dashboardApi = {
  getStats: () => api.get('/api/dashboard/stats'),
  getUsers: () => api.get('/api/dashboard/users'),
  getUsersStatus: () => api.get('/api/dashboard/users-status'),
  getTasks: () => api.get('/api/dashboard/tasks'),
  getCurrentUser: () => api.get('/api/dashboard/me'),
  getEmployees: (year, month) => {
    const params = new URLSearchParams();
    if (year) params.append('year', year);
    if (month) params.append('month', month);
    return api.get(`/api/dashboard/employees?${params.toString()}`);
  },
  getKanban: (userId) => api.get(`/api/dashboard/kanban/${userId}`),
  updateTaskStatus: (taskId, status) => api.post(`/api/dashboard/tasks/${taskId}/status`, { status }),
  assignTask: (taskId, userId) => api.post('/api/dashboard/tasks/assign', { task_id: taskId, user_id: userId }),
  createTask: (name, maxHours, userId, status) => api.post('/api/dashboard/tasks', {
    name,
    max_hours: maxHours,
    user_id: userId,
    status
  }),
  getUserDailySummary: (userId, date) => {
    const params = new URLSearchParams();
    if (userId) params.append('user_id', userId);
    if (date) params.append('date', date);
    return api.get(`/api/dashboard/user-daily-summary?${params.toString()}`);
  },
};
