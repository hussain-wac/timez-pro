import { useNavigate } from 'react-router-dom';
import { Users, ListTodo, Clock } from 'lucide-react';
import { formatHours } from '../utils/format';

export default function ProjectCard({ project }) {
  const navigate = useNavigate();

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

  return (
    <div
      onClick={() => navigate(`/projects/${project.id}`)}
      className="bg-white rounded-2xl shadow-sm p-6 border border-gray-100 cursor-pointer hover:shadow-lg hover:border-blue-200 transition-all group"
    >
      <div className="flex items-start gap-4 mb-4">
        <div className={`w-12 h-12 rounded-xl bg-gradient-to-br ${gradientClass} flex items-center justify-center shadow-sm`}>
          <span className="text-white text-lg font-bold">
            {project.name?.charAt(0).toUpperCase() || 'P'}
          </span>
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-gray-800 text-lg group-hover:text-blue-600 transition-colors truncate">
            {project.name}
          </h3>
          {project.description && (
            <p className="text-sm text-gray-500 line-clamp-2 mt-1">
              {project.description}
            </p>
          )}
        </div>
      </div>

      <div className="grid grid-cols-3 gap-3">
        <div className="bg-gray-50 rounded-lg p-3">
          <div className="flex items-center gap-2 text-gray-500 mb-1">
            <Users className="w-4 h-4" />
            <span className="text-xs">Members</span>
          </div>
          <p className="text-lg font-bold text-gray-800">{project.member_count || 0}</p>
        </div>

        <div className="bg-gray-50 rounded-lg p-3">
          <div className="flex items-center gap-2 text-gray-500 mb-1">
            <ListTodo className="w-4 h-4" />
            <span className="text-xs">Tasks</span>
          </div>
          <p className="text-lg font-bold text-gray-800">{project.task_count || 0}</p>
        </div>

        <div className="bg-gray-50 rounded-lg p-3">
          <div className="flex items-center gap-2 text-gray-500 mb-1">
            <Clock className="w-4 h-4" />
            <span className="text-xs">Hours</span>
          </div>
          <p className="text-lg font-bold text-gray-800">
            {formatHours(project.total_hours || 0)}
          </p>
        </div>
      </div>
    </div>
  );
}
