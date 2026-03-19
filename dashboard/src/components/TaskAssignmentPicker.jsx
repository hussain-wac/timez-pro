import { useState, useEffect } from 'react';
import { X, UserPlus } from 'lucide-react';
import { dashboardApi } from '../api';

export default function TaskAssignmentPicker({ taskId, currentAssignees = [], onUpdate }) {
  const [allUsers, setAllUsers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [selectedUsers, setSelectedUsers] = useState([]);
  const [updating, setUpdating] = useState(false);

  useEffect(() => {
    const fetchUsers = async () => {
      try {
        const data = await dashboardApi.getUsers();
        setAllUsers(data);
      } catch (err) {
        console.error('Failed to fetch users:', err);
      } finally {
        setLoading(false);
      }
    };
    fetchUsers();
  }, []);

  const handleAddAssignees = async () => {
    if (selectedUsers.length === 0) return;
    setUpdating(true);
    try {
      await dashboardApi.assignTaskToUsers(taskId, selectedUsers);
      setShowModal(false);
      setSelectedUsers([]);
      if (onUpdate) onUpdate();
    } catch (err) {
      console.error('Failed to assign users:', err);
      alert('Failed to assign users');
    } finally {
      setUpdating(false);
    }
  };

  const handleRemoveAssignee = async (userId) => {
    try {
      await dashboardApi.unassignTaskUser(taskId, userId);
      if (onUpdate) onUpdate();
    } catch (err) {
      console.error('Failed to remove assignee:', err);
      alert('Failed to remove assignee');
    }
  };

  const toggleUserSelection = (userId) => {
    setSelectedUsers(prev =>
      prev.includes(userId)
        ? prev.filter(id => id !== userId)
        : [...prev, userId]
    );
  };

  const availableUsers = allUsers.filter(
    user => !currentAssignees.some(assignee => assignee.id === user.id)
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium text-gray-700">Assigned To</h4>
        <button
          onClick={() => setShowModal(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
        >
          <UserPlus className="w-4 h-4" />
          Add
        </button>
      </div>

      {currentAssignees.length === 0 ? (
        <p className="text-sm text-gray-400 italic">No one assigned yet</p>
      ) : (
        <div className="flex flex-wrap gap-2">
          {currentAssignees.map(user => (
            <div
              key={user.id}
              className="flex items-center gap-2 px-3 py-1.5 bg-blue-50 text-blue-700 rounded-lg text-sm"
            >
              <div className="w-6 h-6 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center overflow-hidden">
                {user.picture ? (
                  <img src={user.picture} alt={user.name} className="w-full h-full object-cover" />
                ) : (
                  <span className="text-white text-xs font-medium">
                    {user.name?.charAt(0).toUpperCase() || '?'}
                  </span>
                )}
              </div>
              <span className="font-medium">{user.name}</span>
              <button
                onClick={() => handleRemoveAssignee(user.id)}
                className="p-0.5 hover:bg-blue-100 rounded transition-colors"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          ))}
        </div>
      )}

      {showModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl w-full max-w-md shadow-2xl max-h-[80vh] flex flex-col">
            <div className="flex items-center justify-between p-6 border-b border-gray-100">
              <h3 className="text-lg font-semibold text-gray-800">Assign Users</h3>
              <button
                onClick={() => {
                  setShowModal(false);
                  setSelectedUsers([]);
                }}
                className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
              >
                <X className="w-5 h-5 text-gray-500" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-6">
              {loading ? (
                <div className="flex items-center justify-center py-8">
                  <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
                </div>
              ) : availableUsers.length === 0 ? (
                <div className="text-center py-8">
                  <p className="text-gray-500">All users are already assigned</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {availableUsers.map(user => (
                    <label
                      key={user.id}
                      className="flex items-center gap-3 p-3 rounded-lg hover:bg-gray-50 cursor-pointer transition-colors"
                    >
                      <input
                        type="checkbox"
                        checked={selectedUsers.includes(user.id)}
                        onChange={() => toggleUserSelection(user.id)}
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
                        <p className="text-sm text-gray-500 truncate">{user.email}</p>
                      </div>
                    </label>
                  ))}
                </div>
              )}
            </div>

            <div className="flex justify-end gap-3 p-6 border-t border-gray-100">
              <button
                onClick={() => {
                  setShowModal(false);
                  setSelectedUsers([]);
                }}
                className="px-5 py-2.5 text-gray-600 hover:bg-gray-100 rounded-xl transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleAddAssignees}
                disabled={updating || selectedUsers.length === 0}
                className="px-5 py-2.5 bg-blue-600 text-white rounded-xl hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {updating ? 'Assigning...' : `Assign ${selectedUsers.length} User${selectedUsers.length !== 1 ? 's' : ''}`}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
