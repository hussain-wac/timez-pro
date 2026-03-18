import { useState } from 'react';
import { X } from 'lucide-react';

export default function CreateTaskModal({ isOpen, onClose, onSubmit, employees, loading }) {
  const [task, setTask] = useState({ name: '', maxHours: 40, userId: null, status: 'todo' });

  if (!isOpen) return null;

  const handleSubmit = () => {
    if (!task.name || !task.userId) return;
    onSubmit(task);
    setTask({ name: '', maxHours: 40, userId: null, status: 'todo' });
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl w-full max-w-md shadow-2xl">
        <div className="flex items-center justify-between p-6 border-b border-gray-100">
          <h3 className="text-lg font-semibold text-gray-800">Create New Task</h3>
          <button 
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X className="w-5 h-5 text-gray-500" />
          </button>
        </div>
        
        <div className="p-6 space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">Task Name</label>
            <input
              type="text"
              value={task.name}
              onChange={(e) => setTask({ ...task, name: e.target.value })}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
              placeholder="Enter task name"
            />
          </div>
          
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">Max Hours</label>
            <input
              type="number"
              value={task.maxHours}
              onChange={(e) => setTask({ ...task, maxHours: parseInt(e.target.value) || 0 })}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
            />
          </div>
          
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">Assign To</label>
            <select
              value={task.userId || ''}
              onChange={(e) => setTask({ ...task, userId: parseInt(e.target.value) || null })}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
            >
              <option value="">Select employee</option>
              {employees.map(emp => (
                <option key={emp.id} value={emp.id}>{emp.name}</option>
              ))}
            </select>
          </div>
          
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">Status</label>
            <select
              value={task.status}
              onChange={(e) => setTask({ ...task, status: e.target.value })}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
            >
              <option value="todo">To Do</option>
              <option value="in_progress">In Progress</option>
              <option value="review">Review</option>
              <option value="done">Done</option>
            </select>
          </div>
        </div>
        
        <div className="flex justify-end gap-3 p-6 border-t border-gray-100">
          <button 
            onClick={onClose}
            className="px-5 py-2.5 text-gray-600 hover:bg-gray-100 rounded-xl transition-colors"
          >
            Cancel
          </button>
          <button 
            onClick={handleSubmit}
            disabled={loading || !task.name || !task.userId}
            className="px-5 py-2.5 bg-blue-600 text-white rounded-xl hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Creating...' : 'Create Task'}
          </button>
        </div>
      </div>
    </div>
  );
}
