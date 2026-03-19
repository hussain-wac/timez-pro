import { useState, useEffect } from 'react';
import { Calendar, Clock, Users as UsersIcon } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { dashboardApi } from '../api';

export default function Daily() {
  const navigate = useNavigate();
  const [employees, setEmployees] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedDate, setSelectedDate] = useState(() => {
    const today = new Date();
    return today.toISOString().split('T')[0];
  });

  const fetchEmployees = async () => {
    setLoading(true);
    try {
      const currentDate = new Date(selectedDate);
      const year = currentDate.getFullYear();
      const month = currentDate.getMonth() + 1;
      const data = await dashboardApi.getEmployees(year, month);
      setEmployees(data);
    } catch (err) {
      console.error('Failed to fetch employees:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEmployees();
  }, [selectedDate]);

  const formatDate = (dateStr) => {
    const date = new Date(dateStr + 'T00:00:00');
    return date.toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric'
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-800">Daily Work Summary</h1>
          <p className="text-gray-500 mt-1">Select an employee to view their daily work breakdown</p>
        </div>

        <div className="flex items-center gap-2 bg-white rounded-xl p-2 border border-gray-200">
          <Calendar className="w-5 h-5 text-gray-400" />
          <input
            type="date"
            value={selectedDate}
            onChange={(e) => setSelectedDate(e.target.value)}
            max={new Date().toISOString().split('T')[0]}
            className="px-3 py-2 bg-transparent border-none outline-none cursor-pointer text-gray-700 font-medium"
          />
        </div>
      </div>

      <div className="bg-white rounded-2xl p-6 border border-gray-200 shadow-sm">
        <div className="flex items-center gap-3 mb-6">
          <div className="w-10 h-10 rounded-lg bg-blue-100 flex items-center justify-center">
            <Clock className="w-5 h-5 text-blue-600" />
          </div>
          <div>
            <h2 className="text-lg font-semibold text-gray-800">{formatDate(selectedDate)}</h2>
            <p className="text-sm text-gray-500">Select an employee below to view their daily summary</p>
          </div>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      ) : employees.length === 0 ? (
        <div className="bg-white rounded-2xl p-12 text-center border border-gray-100">
          <UsersIcon className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <p className="text-gray-500">No employees found</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {employees.map((employee) => (
            <button
              key={employee.id}
              onClick={() => navigate(`/daily/${employee.id}?date=${selectedDate}`)}
              className="bg-white rounded-2xl shadow-sm p-5 border border-gray-100 hover:shadow-lg hover:border-blue-200 transition-all group text-left"
            >
              <div className="flex items-center gap-4">
                <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center overflow-hidden shadow-md">
                  {employee.picture ? (
                    <img
                      src={employee.picture}
                      alt={employee.name}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <span className="text-white text-xl font-bold">
                      {employee.name?.charAt(0).toUpperCase() || '?'}
                    </span>
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <h3 className="font-semibold text-gray-800 text-lg group-hover:text-blue-600 transition-colors">
                    {employee.name || 'Unknown'}
                  </h3>
                  <p className="text-sm text-gray-500 truncate">{employee.email}</p>
                  <div className="flex items-center gap-4 mt-2">
                    <span className="text-xs text-gray-400 flex items-center gap-1">
                      <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                      {employee.task_count} tasks
                    </span>
                  </div>
                </div>
                <div className="text-right">
                  <p className="text-sm text-gray-500">View Details</p>
                  <p className="text-xs text-gray-400 mt-1">&rarr;</p>
                </div>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
