import { useState, useEffect } from 'react';
import { Users } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { dashboardApi } from '../api';
import ProjectSelector from '../components/ProjectSelector';

const months = ['January', 'February', 'March', 'April', 'May', 'June',
                'July', 'August', 'September', 'October', 'November', 'December'];

export default function Employees() {
  const navigate = useNavigate();
  const [employees, setEmployees] = useState([]);
  const [loading, setLoading] = useState(true);
  const [year, setYear] = useState(() => new Date().getFullYear());
  const [month, setMonth] = useState(() => new Date().getMonth() + 1);
  const [selectedProject, setSelectedProject] = useState(null);

  const fetchEmployees = async () => {
    setLoading(true);
    try {
      const data = await dashboardApi.getEmployees(year, month);
      setEmployees(data);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEmployees();
  }, [year, month]);

  const currentMonthName = months[month - 1];

  // Filter employees by project if selected
  const filteredEmployees = selectedProject
    ? employees.filter(emp => emp.projects?.some(p => p.id === parseInt(selectedProject)))
    : employees;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">Employees</h1>
          <p className="text-gray-600 mt-1 text-sm">View employee working hours and tasks</p>
        </div>

        <div className="flex items-center gap-3">
          <ProjectSelector
            value={selectedProject}
            onChange={setSelectedProject}
            className="w-48"
          />
          <div className="flex items-center gap-2 bg-white rounded-md border border-gray-300">
            <select
              value={month}
              onChange={(e) => setMonth(parseInt(e.target.value))}
              className="px-3 py-2 bg-transparent border-none outline-none cursor-pointer text-sm"
            >
              {months.map((m, i) => (
                <option key={i + 1} value={i + 1}>{m}</option>
              ))}
            </select>
            <span className="text-gray-300">|</span>
            <select
              value={year}
              onChange={(e) => setYear(parseInt(e.target.value))}
              className="px-3 py-2 bg-transparent border-none outline-none cursor-pointer text-sm"
            >
              {[2024, 2025, 2026].map(y => (
                <option key={y} value={y}>{y}</option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      ) : filteredEmployees.length === 0 ? (
        <div className="bg-white rounded-md p-12 text-center border border-gray-200">
          <Users className="w-12 h-12 text-gray-400 mx-auto mb-3" />
          <p className="text-gray-600 text-sm">
            {selectedProject ? 'No employees found in this project' : 'No employees found'}
          </p>
        </div>
      ) : (
        <div className="bg-white rounded-md border border-gray-200 overflow-hidden">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Employee
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Email
                </th>
                <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Tasks
                </th>
                <th className="px-6 py-3 text-center text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Monthly Hours
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {filteredEmployees.map(employee => (
                <tr
                  key={employee.id}
                  onClick={() => navigate(`/employees/${employee.id}`)}
                  className="hover:bg-gray-50 cursor-pointer transition-colors"
                >
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-md bg-blue-500 flex items-center justify-center overflow-hidden flex-shrink-0">
                        {employee.picture ? (
                          <img src={employee.picture} alt={employee.name} className="w-full h-full object-cover" />
                        ) : (
                          <span className="text-white text-sm font-semibold">
                            {employee.name?.charAt(0).toUpperCase() || '?'}
                          </span>
                        )}
                      </div>
                      <span className="text-sm font-medium text-gray-900">{employee.name || 'Unknown'}</span>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className="text-sm text-gray-600">{employee.email}</span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-center">
                    <span className="inline-flex items-center px-2.5 py-0.5 rounded-md text-xs font-medium bg-gray-100 text-gray-800">
                      {employee.task_count} tasks
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-center">
                    <span className="text-sm font-semibold text-blue-600">{employee.monthly_hours}h</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
