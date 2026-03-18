import { GripVertical, Clock, Lock } from 'lucide-react';
import { formatHours } from '../utils/format';

const statusOrder = ['todo', 'in_progress', 'review', 'done'];

export default function KanbanColumn({ title, tasks, onStatusChange, countColor, currentStatus }) {
  const currentIndex = statusOrder.indexOf(currentStatus);

  const handleDragOver = (e) => {
    e.preventDefault();
    e.currentTarget.classList.add('bg-gray-100');
  };

  const handleDragLeave = (e) => {
    e.currentTarget.classList.remove('bg-gray-100');
  };

  const handleDrop = (e) => {
    e.preventDefault();
    e.currentTarget.classList.remove('bg-gray-100');
    
    const taskId = parseInt(e.dataTransfer.getData('taskId'));
    const taskStatus = e.dataTransfer.getData('taskStatus');
    const taskIndex = statusOrder.indexOf(taskStatus);
    
    if (currentIndex > taskIndex) {
      onStatusChange(taskId, currentStatus);
    }
  };

  return (
    <div 
      className="flex-1 min-w-[300px] bg-gray-50/50 rounded-2xl p-4 border-2 border-dashed border-gray-100"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <h3 className="font-semibold text-gray-700">{title}</h3>
          <span className={`px-2.5 py-1 ${countColor} text-xs font-medium rounded-full`}>
            {tasks.length}
          </span>
        </div>
      </div>
      <div className="space-y-3 max-h-[calc(100vh-300px)] overflow-y-auto">
        {tasks.map(task => {
          return (
            <div
              key={task.id}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData('taskId', task.id);
                e.dataTransfer.setData('taskStatus', task.status || 'todo');
              }}
              className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 cursor-grab active:cursor-grabbing hover:shadow-md transition-all group"
            >
              <div className="flex items-start justify-between mb-2">
                <h4 className="font-medium text-gray-800 flex-1">
                  {task.name}
                </h4>
                <GripVertical className="w-4 h-4 text-gray-300 group-hover:text-gray-500" />
              </div>
              <div className="flex items-center justify-between text-sm">
                <span className="text-gray-500 flex items-center gap-1">
                  <Clock className="w-3 h-3" />
                  {task.max_hours}h goal
                </span>
                <span className="text-blue-600 font-medium">{formatHours(task.total_tracked_seconds || 0)}</span>
              </div>
            </div>
          );
        })}
        {tasks.length === 0 && (
          <div className="text-center py-12 text-gray-400">
            <p className="text-sm">No tasks</p>
          </div>
        )}
      </div>
    </div>
  );
}
