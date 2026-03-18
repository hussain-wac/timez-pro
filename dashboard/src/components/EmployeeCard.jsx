import { useNavigate } from 'react-router-dom';

export default function EmployeeCard({ employee }) {
  const navigate = useNavigate();

  return (
    <div 
      onClick={() => navigate(`/employees/${employee.id}`)}
      className="bg-white rounded-2xl shadow-sm p-5 border border-gray-100 cursor-pointer hover:shadow-lg hover:border-blue-200 transition-all group"
    >
      <div className="flex items-center gap-4">
        <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center overflow-hidden shadow-md">
          {employee.picture ? (
            <img src={employee.picture} alt={employee.name} className="w-full h-full object-cover" />
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
          <p className="text-2xl font-bold text-blue-600">{employee.monthly_hours}h</p>
          <p className="text-xs text-gray-400">this month</p>
        </div>
      </div>
    </div>
  );
}
