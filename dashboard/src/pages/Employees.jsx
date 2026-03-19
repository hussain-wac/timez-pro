import { useState, useEffect } from 'react';
import { Users, ChevronDown } from 'lucide-react';
import { dashboardApi } from '../api';
import EmployeeCard from '../components/EmployeeCard';
import ProjectSelector from '../components/ProjectSelector';

const months = ['January', 'February', 'March', 'April', 'May', 'June', 
                'July', 'August', 'September', 'October', 'November', 'December'];

export default function Employees() {
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
          <h1 className="text-2xl font-bold text-gray-800">Employees</h1>
          <p className="text-gray-500 mt-1">View employee working hours and tasks</p>
        </div>

        <div className="flex items-center gap-3">
          <ProjectSelector
            value={selectedProject}
            onChange={setSelectedProject}
            className="w-48"
          />
          <div className="flex items-center gap-2 bg-white rounded-xl p-1 border border-gray-200">
            <select
              value={month}
              onChange={(e) => setMonth(parseInt(e.target.value))}
              className="px-4 py-2 bg-transparent border-none outline-none cursor-pointer"
            >
              {months.map((m, i) => (
                <option key={i + 1} value={i + 1}>{m}</option>
              ))}
            </select>
            <span className="text-gray-300">|</span>
            <select
              value={year}
              onChange={(e) => setYear(parseInt(e.target.value))}
              className="px-4 py-2 bg-transparent border-none outline-none cursor-pointer"
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
        <div className="bg-white rounded-2xl p-12 text-center border border-gray-100">
          <Users className="w-12 h-12 text-gray-300 mx-auto mb-4" />
          <p className="text-gray-500">
            {selectedProject ? 'No employees found in this project' : 'No employees found'}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredEmployees.map(employee => (
            <EmployeeCard key={employee.id} employee={employee} />
          ))}
        </div>
      )}
    </div>
  );
}
