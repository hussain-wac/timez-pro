import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus, Kanban as KanbanIcon } from 'lucide-react';
import { dashboardApi } from '../api';
import KanbanColumn from '../components/KanbanColumn';
import CreateTaskModal from '../components/CreateTaskModal';
import ProjectSelector from '../components/ProjectSelector';

const statusColumns = [
  { key: 'todo', title: 'To Do', countColor: 'bg-gray-100 text-gray-600' },
  { key: 'in_progress', title: 'In Progress', countColor: 'bg-blue-100 text-blue-600' },
  { key: 'review', title: 'Review', countColor: 'bg-yellow-100 text-yellow-600' },
  { key: 'done', title: 'Done', countColor: 'bg-green-100 text-green-600' },
];

export default function Kanban() {
  const { userId } = useParams();
  const navigate = useNavigate();
  const [kanban, setKanban] = useState({ todo: [], in_progress: [], review: [], done: [] });
  const [loading, setLoading] = useState(true);
  const [employees, setEmployees] = useState([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [creating, setCreating] = useState(false);
  const [selectedProject, setSelectedProject] = useState(null);

  const fetchKanban = async () => {
    if (!userId) return;
    setLoading(true);
    try {
      const data = await dashboardApi.getKanban(parseInt(userId));
      setKanban(data);
    } catch (err) {
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const fetchEmployees = async () => {
    try {
      const data = await dashboardApi.getEmployees();
      setEmployees(data);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    fetchKanban();
    fetchEmployees();
  }, [userId]);

  const handleStatusChange = async (taskId, status) => {
    try {
      await dashboardApi.updateTaskStatus(taskId, status);
      fetchKanban();
    } catch (err) {
      console.error(err);
    }
  };

  const handleCreateTask = async (task) => {
    setCreating(true);
    try {
      await dashboardApi.createTask(task.name, task.maxHours, task.userId, task.status);
      setShowCreateModal(false);
      fetchKanban();
    } catch (err) {
      console.error(err);
    } finally {
      setCreating(false);
    }
  };

  const selectedEmployee = employees.find(e => e.id === parseInt(userId));

  // Filter tasks by project if selected
  const filterTasksByProject = (tasks) => {
    if (!selectedProject) return tasks;
    return tasks.filter(task => task.project_id === parseInt(selectedProject));
  };

  const filteredKanban = {
    todo: filterTasksByProject(kanban.todo),
    in_progress: filterTasksByProject(kanban.in_progress),
    review: filterTasksByProject(kanban.review),
    done: filterTasksByProject(kanban.done),
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <button
            onClick={() => navigate('/employees')}
            className="flex items-center gap-2 text-gray-500 hover:text-gray-700 mb-2 transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            <span className="text-sm">Back to Employees</span>
          </button>
          <h1 className="text-2xl font-bold text-gray-800">
            {selectedEmployee?.name || 'Kanban Board'}
          </h1>
          <p className="text-gray-500 mt-1">Drag and drop tasks to change status</p>
        </div>
        <div className="flex items-center gap-3">
          <ProjectSelector
            value={selectedProject}
            onChange={setSelectedProject}
            className="w-48"
          />
          <button
            onClick={() => setShowCreateModal(true)}
            className="flex items-center gap-2 px-5 py-2.5 bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors shadow-sm"
          >
            <Plus className="w-5 h-5" />
            Create Task
          </button>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center h-64">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        </div>
      ) : (
        <div className="flex gap-4 overflow-x-auto pb-4">
          {statusColumns.map(col => (
            <KanbanColumn
              key={col.key}
              title={col.title}
              currentStatus={col.key}
              tasks={filteredKanban[col.key] || []}
              onStatusChange={handleStatusChange}
              countColor={col.countColor}
            />
          ))}
        </div>
      )}

      <CreateTaskModal 
        isOpen={showCreateModal} 
        onClose={() => setShowCreateModal(false)}
        onSubmit={handleCreateTask}
        employees={employees}
        loading={creating}
      />
    </div>
  );
}
