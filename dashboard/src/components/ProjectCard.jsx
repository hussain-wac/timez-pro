import { useNavigate } from 'react-router-dom';
import { Users, ListTodo, Clock } from 'lucide-react';
import { formatHours } from '../utils/format';

export default function ProjectCard({ project }) {
  const navigate = useNavigate();

  const colorClasses = {
    blue: 'bg-blue-500',
    purple: 'bg-purple-500',
    green: 'bg-green-500',
    orange: 'bg-orange-500',
    pink: 'bg-pink-500',
    indigo: 'bg-indigo-500',
    red: 'bg-red-500',
    teal: 'bg-teal-500',
  };

  const colorClass = colorClasses[project.color] || colorClasses.blue;

  return (
    <div
      onClick={() => navigate(`/projects/${project.id}`)}
      className="bg-white rounded-md border border-gray-200 p-4 cursor-pointer hover:border-blue-400 transition-colors group"
    >
      <div className="flex items-start gap-3 mb-4">
        <div className={`w-10 h-10 rounded-md ${colorClass} flex items-center justify-center flex-shrink-0`}>
          <span className="text-white text-sm font-semibold">
            {project.name?.charAt(0).toUpperCase() || 'P'}
          </span>
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-medium text-gray-900 text-sm group-hover:text-blue-600 transition-colors truncate">
            {project.name}
          </h3>
          {project.description && (
            <p className="text-xs text-gray-600 line-clamp-2 mt-1">
              {project.description}
            </p>
          )}
        </div>
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between text-xs">
          <span className="text-gray-600">Members</span>
          <span className="font-medium text-gray-900">{project.member_count || 0}</span>
        </div>
        <div className="flex items-center justify-between text-xs">
          <span className="text-gray-600">Tasks</span>
          <span className="font-medium text-gray-900">{project.task_count || 0}</span>
        </div>
        <div className="flex items-center justify-between text-xs">
          <span className="text-gray-600">Hours</span>
          <span className="font-medium text-gray-900">{formatHours(project.total_hours || 0)}</span>
        </div>
      </div>
    </div>
  );
}
