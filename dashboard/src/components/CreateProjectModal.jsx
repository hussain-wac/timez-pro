import { useState, useEffect } from 'react';
import { X, Users } from 'lucide-react';
import { dashboardApi } from '../api';

const COLORS = [
  { name: 'Blue', value: 'blue', class: 'bg-gradient-to-br from-blue-400 to-blue-600' },
  { name: 'Purple', value: 'purple', class: 'bg-gradient-to-br from-purple-400 to-purple-600' },
  { name: 'Green', value: 'green', class: 'bg-gradient-to-br from-green-400 to-green-600' },
  { name: 'Orange', value: 'orange', class: 'bg-gradient-to-br from-orange-400 to-orange-600' },
  { name: 'Pink', value: 'pink', class: 'bg-gradient-to-br from-pink-400 to-pink-600' },
  { name: 'Indigo', value: 'indigo', class: 'bg-gradient-to-br from-indigo-400 to-indigo-600' },
  { name: 'Red', value: 'red', class: 'bg-gradient-to-br from-red-400 to-red-600' },
  { name: 'Teal', value: 'teal', class: 'bg-gradient-to-br from-teal-400 to-teal-600' },
];

export default function CreateProjectModal({ isOpen, onClose, onSuccess, editProject = null }) {
  const [project, setProject] = useState({
    name: '',
    description: '',
    color: 'blue',
  });
  const [selectedMembers, setSelectedMembers] = useState([]);
  const [allUsers, setAllUsers] = useState([]);
  const [loading, setLoading] = useState(false);
  const [usersLoading, setUsersLoading] = useState(true);

  useEffect(() => {
    if (isOpen) {
      fetchUsers();
      if (editProject) {
        setProject({
          name: editProject.name || '',
          description: editProject.description || '',
          color: editProject.color || 'blue',
        });
        setSelectedMembers([]);
      } else {
        setProject({ name: '', description: '', color: 'blue' });
        setSelectedMembers([]);
      }
    }
  }, [isOpen, editProject]);

  const fetchUsers = async () => {
    setUsersLoading(true);
    try {
      const data = await dashboardApi.getUsers();
      setAllUsers(data);
    } catch (err) {
      console.error('Failed to fetch users:', err);
    } finally {
      setUsersLoading(false);
    }
  };

  const handleSubmit = async () => {
    if (!project.name.trim()) {
      alert('Please enter a project name');
      return;
    }

    setLoading(true);
    try {
      if (editProject) {
        await dashboardApi.updateProject(editProject.id, project);
      } else {
        const newProject = await dashboardApi.createProject(project);
        if (selectedMembers.length > 0) {
          await dashboardApi.addProjectMembers(newProject.id, selectedMembers);
        }
      }
      onSuccess();
      onClose();
    } catch (err) {
      console.error('Failed to save project:', err);
      alert('Failed to save project');
    } finally {
      setLoading(false);
    }
  };

  const toggleMemberSelection = (userId) => {
    setSelectedMembers(prev =>
      prev.includes(userId)
        ? prev.filter(id => id !== userId)
        : [...prev, userId]
    );
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl w-full max-w-2xl shadow-2xl max-h-[90vh] flex flex-col">
        <div className="flex items-center justify-between p-6 border-b border-gray-100">
          <h3 className="text-lg font-semibold text-gray-800">
            {editProject ? 'Edit Project' : 'Create New Project'}
          </h3>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X className="w-5 h-5 text-gray-500" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Project Name
            </label>
            <input
              type="text"
              value={project.name}
              onChange={(e) => setProject({ ...project, name: e.target.value })}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
              placeholder="Enter project name"
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Description
            </label>
            <textarea
              value={project.description}
              onChange={(e) => setProject({ ...project, description: e.target.value })}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all resize-none"
              placeholder="Enter project description"
              rows={3}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Project Color
            </label>
            <div className="grid grid-cols-4 gap-3">
              {COLORS.map(color => (
                <button
                  key={color.value}
                  onClick={() => setProject({ ...project, color: color.value })}
                  className={`relative h-16 rounded-xl ${color.class} transition-all ${
                    project.color === color.value
                      ? 'ring-4 ring-blue-500 ring-offset-2'
                      : 'hover:scale-105'
                  }`}
                >
                  {project.color === color.value && (
                    <div className="absolute inset-0 flex items-center justify-center">
                      <div className="w-6 h-6 bg-white rounded-full flex items-center justify-center">
                        <div className="w-3 h-3 bg-blue-600 rounded-full"></div>
                      </div>
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>

          {!editProject && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-3">
                Initial Members (optional)
              </label>
              {usersLoading ? (
                <div className="flex items-center justify-center py-8">
                  <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
                </div>
              ) : (
                <div className="border border-gray-200 rounded-xl p-4 max-h-64 overflow-y-auto space-y-2">
                  {allUsers.map(user => (
                    <label
                      key={user.id}
                      className="flex items-center gap-3 p-2 rounded-lg hover:bg-gray-50 cursor-pointer transition-colors"
                    >
                      <input
                        type="checkbox"
                        checked={selectedMembers.includes(user.id)}
                        onChange={() => toggleMemberSelection(user.id)}
                        className="w-4 h-4 text-blue-600 rounded focus:ring-2 focus:ring-blue-500"
                      />
                      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center overflow-hidden flex-shrink-0">
                        {user.picture ? (
                          <img src={user.picture} alt={user.name} className="w-full h-full object-cover" />
                        ) : (
                          <span className="text-white text-sm font-medium">
                            {user.name?.charAt(0).toUpperCase() || '?'}
                          </span>
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-medium text-gray-800 truncate">{user.name}</p>
                        <p className="text-xs text-gray-500 truncate">{user.email}</p>
                      </div>
                    </label>
                  ))}
                </div>
              )}
              {selectedMembers.length > 0 && (
                <p className="text-sm text-gray-500 mt-2">
                  {selectedMembers.length} member{selectedMembers.length !== 1 ? 's' : ''} selected
                </p>
              )}
            </div>
          )}
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
            disabled={loading || !project.name.trim()}
            className="px-5 py-2.5 bg-blue-600 text-white rounded-xl hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? 'Saving...' : editProject ? 'Save Changes' : 'Create Project'}
          </button>
        </div>
      </div>
    </div>
  );
}
