import { useState, useEffect, useCallback, useRef } from 'react';
import { UserPlus, X, Users, AlertTriangle } from 'lucide-react';
import { dashboardApi } from '../api';

export default function MemberManager({ projectId }) {
  const [members, setMembers] = useState([]);
  const [allUsers, setAllUsers] = useState([]);
  const [loading, setLoading] = useState(true);
  const [showAddModal, setShowAddModal] = useState(false);
  const [selectedUsers, setSelectedUsers] = useState([]);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState(null);
  // Confirmation modal state
  const [showRemoveConfirm, setShowRemoveConfirm] = useState(false);
  const [memberToRemove, setMemberToRemove] = useState(null);
  const [removing, setRemoving] = useState(false);

  // Track mount state to prevent state updates after unmount
  const isMountedRef = useRef(true);
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const fetchMembers = useCallback(async () => {
    try {
      const data = await dashboardApi.getProjectMembers(projectId);
      if (isMountedRef.current) {
        setMembers(data);
      }
    } catch (err) {
      console.error('Failed to fetch members:', err);
    }
  }, [projectId]);

  const fetchAllUsers = useCallback(async () => {
    try {
      const data = await dashboardApi.getUsers();
      if (isMountedRef.current) {
        setAllUsers(data);
      }
    } catch (err) {
      console.error('Failed to fetch users:', err);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const loadData = async () => {
      setLoading(true);
      await Promise.all([fetchMembers(), fetchAllUsers()]);
      if (!cancelled) {
        setLoading(false);
      }
    };
    loadData();
    return () => {
      cancelled = true;
    };
  }, [projectId, fetchMembers, fetchAllUsers]);

  const handleRemoveMemberClick = useCallback((member) => {
    setMemberToRemove(member);
    setShowRemoveConfirm(true);
  }, []);

  const handleConfirmRemove = useCallback(async () => {
    if (!memberToRemove) return;

    setRemoving(true);
    setError(null);
    try {
      await dashboardApi.removeProjectMember(projectId, memberToRemove.user_id);
      if (isMountedRef.current) {
        await fetchMembers();
        setShowRemoveConfirm(false);
        setMemberToRemove(null);
      }
    } catch (err) {
      console.error('Failed to remove member:', err);
      if (isMountedRef.current) {
        setError('Failed to remove member. Please try again.');
      }
    } finally {
      if (isMountedRef.current) {
        setRemoving(false);
      }
    }
  }, [memberToRemove, projectId, fetchMembers]);

  const handleCancelRemove = useCallback(() => {
    setShowRemoveConfirm(false);
    setMemberToRemove(null);
    setError(null);
  }, []);

  const handleAddMembers = useCallback(async () => {
    if (selectedUsers.length === 0) return;
    setAdding(true);
    setError(null);
    try {
      await dashboardApi.addProjectMembers(projectId, selectedUsers);
      if (isMountedRef.current) {
        await fetchMembers();
        setShowAddModal(false);
        setSelectedUsers([]);
      }
    } catch (err) {
      console.error('Failed to add members:', err);
      if (isMountedRef.current) {
        setError('Failed to add members. Please try again.');
      }
    } finally {
      if (isMountedRef.current) {
        setAdding(false);
      }
    }
  }, [selectedUsers, projectId, fetchMembers]);

  const availableUsers = allUsers.filter(
    user => !members.some(member => member.user_id === user.id)
  );

  const toggleUserSelection = useCallback((userId) => {
    setSelectedUsers(prev =>
      prev.includes(userId)
        ? prev.filter(id => id !== userId)
        : [...prev, userId]
    );
  }, []);

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
        <h3 className="text-base font-medium text-gray-900">Project Members</h3>
        <button
          onClick={() => setShowAddModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors text-sm font-medium"
        >
          <UserPlus className="w-4 h-4" />
          Add Members
        </button>
      </div>

      {members.length === 0 ? (
        <div className="bg-gray-50 rounded-md p-12 text-center">
          <Users className="w-12 h-12 text-gray-400 mx-auto mb-3" />
          <p className="text-gray-600 text-sm">No members yet</p>
          <p className="text-xs text-gray-500 mt-1">Add members to get started</p>
        </div>
      ) : (
        <div className="border border-gray-200 rounded-md overflow-hidden">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Member
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Email
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {members.map(member => (
                <tr key={member.id} className="hover:bg-gray-50">
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-md bg-blue-500 flex items-center justify-center overflow-hidden flex-shrink-0">
                        {member.user?.picture ? (
                          <img src={member.user.picture} alt={member.user?.name || ''} className="w-full h-full object-cover" />
                        ) : (
                          <span className="text-white text-xs font-semibold">
                            {member.user?.name?.charAt(0).toUpperCase() || member.user?.email?.charAt(0).toUpperCase() || '?'}
                          </span>
                        )}
                      </div>
                      <div className="flex flex-col">
                        <span className="text-sm font-medium text-gray-900">{member.user?.name || 'Unknown'}</span>
                        {member.role === 'lead' && (
                          <span className="text-xs text-blue-600">Lead</span>
                        )}
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className="text-sm text-gray-600">{member.user?.email || '-'}</span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-right">
                    <button
                      onClick={() => handleRemoveMemberClick(member)}
                      className="inline-flex items-center gap-1 px-3 py-1 text-xs font-medium text-red-600 hover:text-red-700 hover:bg-red-50 rounded-md transition-colors"
                    >
                      <X className="w-3 h-3" />
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Remove Member Confirmation Modal */}
      {showRemoveConfirm && memberToRemove && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-md w-full max-w-sm shadow-xl mx-4">
            <div className="p-6">
              <div className="flex items-center gap-3 mb-4">
                <div className="w-10 h-10 bg-red-100 rounded-full flex items-center justify-center flex-shrink-0">
                  <AlertTriangle className="w-5 h-5 text-red-600" />
                </div>
                <div>
                  <h3 className="text-base font-medium text-gray-900">Remove Member</h3>
                  <p className="text-sm text-gray-500">This action cannot be undone.</p>
                </div>
              </div>
              <p className="text-sm text-gray-600 mb-4">
                Are you sure you want to remove <span className="font-medium">{memberToRemove.user?.name || memberToRemove.user?.email || 'this member'}</span> from this project?
              </p>

              {error && (
                <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md text-sm text-red-700">
                  {error}
                </div>
              )}

              <div className="flex justify-end gap-3">
                <button
                  onClick={handleCancelRemove}
                  disabled={removing}
                  className="px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 rounded-md transition-colors font-medium disabled:opacity-50"
                >
                  Cancel
                </button>
                <button
                  onClick={handleConfirmRemove}
                  disabled={removing}
                  className="px-4 py-2 text-sm bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 transition-colors font-medium"
                >
                  {removing ? 'Removing...' : 'Remove'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Add Members Modal */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-md w-full max-w-md shadow-xl max-h-[80vh] flex flex-col mx-4">
            <div className="flex items-center justify-between p-6 border-b border-gray-200">
              <h3 className="text-base font-medium text-gray-900">Add Members</h3>
              <button
                onClick={() => {
                  setShowAddModal(false);
                  setSelectedUsers([]);
                  setError(null);
                }}
                className="p-1 hover:bg-gray-100 rounded transition-colors"
              >
                <X className="w-5 h-5 text-gray-500" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-6">
              {error && (
                <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md text-sm text-red-700">
                  {error}
                </div>
              )}

              {availableUsers.length === 0 ? (
                <div className="text-center py-8">
                  <p className="text-gray-600 text-sm">All users are already members</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {availableUsers.map(user => (
                    <label
                      key={user.id}
                      className="flex items-center gap-3 p-3 rounded-md hover:bg-gray-50 cursor-pointer transition-colors"
                    >
                      <input
                        type="checkbox"
                        checked={selectedUsers.includes(user.id)}
                        onChange={() => toggleUserSelection(user.id)}
                        className="w-4 h-4 text-blue-600 rounded focus:ring-2 focus:ring-blue-500"
                      />
                      <div className="w-8 h-8 rounded-md bg-blue-500 flex items-center justify-center overflow-hidden flex-shrink-0">
                        {user.picture ? (
                          <img src={user.picture} alt={user.name} className="w-full h-full object-cover" />
                        ) : (
                          <span className="text-white text-xs font-semibold">
                            {user.name?.charAt(0).toUpperCase() || '?'}
                          </span>
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-medium text-gray-900 text-sm truncate">{user.name}</p>
                        <p className="text-xs text-gray-600 truncate">{user.email}</p>
                      </div>
                    </label>
                  ))}
                </div>
              )}
            </div>

            <div className="flex justify-end gap-3 p-6 border-t border-gray-200">
              <button
                onClick={() => {
                  setShowAddModal(false);
                  setSelectedUsers([]);
                  setError(null);
                }}
                className="px-4 py-2 text-sm text-gray-700 hover:bg-gray-100 rounded-md transition-colors font-medium"
              >
                Cancel
              </button>
              <button
                onClick={handleAddMembers}
                disabled={adding || selectedUsers.length === 0}
                className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors font-medium"
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
