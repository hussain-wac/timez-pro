import { useState, useEffect } from 'react';
import { UserPlus, X, Users } from 'lucide-react';
import { dashboardApi } from '../api';

export default function MemberManager({ projectId }) {
  const [members, setMembers] = useState([]);
  const [allUsers, setAllUsers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedUsers, setSelectedUsers] = useState([]);
  const [adding, setAdding] = useState(false);

  const fetchMembers = async () => {
    try {
      const data = await dashboardApi.getProjectMembers(projectId);
      setMembers(data);
    } catch (err) {
      console.error('Failed to fetch members:', err);
    }
  };

  const fetchAllUsers = async () => {
    try {
      const data = await dashboardApi.getUsers();
      setAllUsers(data);
    } catch (err) {
      console.error('Failed to fetch users:', err);
    }
  };

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      await Promise.all([fetchMembers(), fetchAllUsers()]);
      setLoading(false);
    };
    loadData();
  }, [projectId]);

  const handleRemoveMember = async (userId) => {
    if (!confirm('Remove this member from the project?')) return;
    try {
      await dashboardApi.removeProjectMember(projectId, userId);
      await fetchMembers();
    } catch (err) {
      console.error('Failed to remove member:', err);
      alert('Failed to remove member');
    }
  };

  const handleAddMembers = async () => {
    if (selectedUsers.length === 0) return;
    setAdding(true);
    try {
      await dashboardApi.addProjectMembers(projectId, selectedUsers);
      await fetchMembers();
      setShowAddModal(false);
      setSelectedUsers([]);
    } catch (err) {
      console.error('Failed to add members:', err);
      alert('Failed to add members');
    } finally {
      setAdding(false);
    }
  };

  const availableUsers = allUsers.filter(
    user => !members.some(member => member.id === user.id)
  );

  const toggleUserSelection = (userId) => {
    setSelectedUsers(prev =>
      prev.includes(userId)
        ? prev.filter(id => id !== userId)
        : [...prev, userId]
    );
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-800">Project Members</h3>
        <button
          onClick={() => setShowAddModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm"
        >
          <UserPlus className="w-4 h-4" />
          Add Members
        </button>
      </div>

      {members.length === 0 ? (
        <div className="bg-gray-50 rounded-xl p-12 text-center">
          <Users className="w-12 h-12 text-gray-300 mx-auto mb-3" />
          <p className="text-gray-500">No members yet</p>
          <p className="text-sm text-gray-400 mt-1">Add members to get started</p>
        </div>
      ) : (
        <div className="space-y-2">
          {members.map(member => (
            <div
              key={member.id}
              className="flex items-center justify-between p-4 bg-white rounded-xl border border-gray-100 hover:shadow-sm transition-shadow"
            >
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center overflow-hidden">
                  {member.picture ? (
                    <img src={member.picture} alt={member.name} className="w-full h-full object-cover" />
                  ) : (
                    <span className="text-white font-medium">
                      {member.name?.charAt(0).toUpperCase() || '?'}
                    </span>
                  )}
                </div>
                <div>
                  <p className="font-medium text-gray-800">{member.name}</p>
                  <p className="text-sm text-gray-500">{member.email}</p>
                </div>
              </div>
              <button
                onClick={() => handleRemoveMember(member.id)}
                className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          ))}
        </div>
      )}

      {showAddModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl w-full max-w-md shadow-2xl max-h-[80vh] flex flex-col">
            <div className="flex items-center justify-between p-6 border-b border-gray-100">
              <h3 className="text-lg font-semibold text-gray-800">Add Members</h3>
              <button
                onClick={() => {
                  setShowAddModal(false);
                  setSelectedUsers([]);
                }}
                className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
              >
                <X className="w-5 h-5 text-gray-500" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-6">
              {availableUsers.length === 0 ? (
                <div className="text-center py-8">
                  <p className="text-gray-500">All users are already members</p>
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
                  setShowAddModal(false);
                  setSelectedUsers([]);
                }}
                className="px-5 py-2.5 text-gray-600 hover:bg-gray-100 rounded-xl transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleAddMembers}
                disabled={adding || selectedUsers.length === 0}
                className="px-5 py-2.5 bg-blue-600 text-white rounded-xl hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {adding ? 'Adding...' : `Add ${selectedUsers.length} Member${selectedUsers.length !== 1 ? 's' : ''}`}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
