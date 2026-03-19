# Timez Pro Dashboard - Project Management Implementation

## Project Overview
Implemented comprehensive project management features for Timez Pro admin dashboard. The dashboard uses React 19, React Router v7, Tailwind CSS, and follows component composition patterns.

## Architecture Patterns

### Component Structure
- **Pages**: Full-page components in `/src/pages/` (DashboardHome, Projects, ProjectDetail, Employees, Kanban)
- **Components**: Reusable UI in `/src/components/` (cards, modals, selectors, managers)
- **API Layer**: Centralized in `/src/api.js` with `dashboardApi` namespace
- **Utilities**: Helper functions in `/src/utils/` (format.js for time formatting)

### State Management
- Local state with `useState` for component-specific data
- No global state library - passing props and callbacks
- API calls made at page level, data flows down to components
- Refetch patterns: callbacks passed to modals (onSuccess) trigger parent refetch

### Component Patterns Used

#### Cards
- `ProjectCard`, `EmployeeCard`, `StatCard` - consistent card design
- Gradient avatars/icons with color variants
- Hover effects with `group` class and transitions
- Click handlers for navigation

#### Modals
- Fixed overlay with `fixed inset-0 bg-black/50`
- Centered white rounded cards with `max-w-md` or `max-w-2xl`
- Header/body/footer structure with borders
- Disabled states during API calls
- Reset form state on close

#### Selectors/Pickers
- `ProjectSelector` - dropdown for filtering
- `MemberManager` - list + add modal pattern
- `TaskAssignmentPicker` - similar add/remove pattern
- Checkbox lists for multi-select

### API Integration
- All endpoints in `dashboardApi` object
- Pattern: `getX()`, `createX(data)`, `updateX(id, data)`, `deleteX(id)`
- Nested resources: `getProjectMembers(projectId)`, `addProjectMembers(projectId, userIds)`
- Auth handled in `getAuthHeaders()` helper

### Styling Conventions (UPDATED - Clean Admin Design)
- Tailwind utility classes, no custom CSS
- **Border Radius**: Maximum `rounded-md` (not `rounded-xl`, `rounded-2xl`)
- **Shadows**: Minimal - use borders instead (`border border-gray-200`)
- **Colors**:
  - Primary: Blue-600 (#2563eb)
  - Avatars: Solid colors `bg-blue-500` (NO gradients)
  - Text: `text-gray-900` (headings), `text-gray-600` (body), `text-gray-500` (muted)
  - Borders: `border-gray-200` (containers), `border-gray-300` (inputs)
- **Spacing**: `gap-4` for grids, `space-y-6` for vertical sections
- **Transitions**: `transition-colors` on interactive elements
- **Typography**:
  - Headings: `text-2xl font-semibold text-gray-900`
  - Subheadings: `text-base font-medium text-gray-900`
  - Body: `text-sm text-gray-600`
  - Labels: `text-sm font-medium text-gray-700`

### Navigation
- `useNavigate()` hook for programmatic navigation
- `NavLink` in Layout with active state styling
- Back buttons with `ArrowLeft` icon
- Routes defined in App.jsx

## Key Features Implemented

### Projects List Page
- Grid of project cards with stats (members, tasks, hours)
- Create project button
- Empty state with CTA
- Color-coded project indicators

### Project Detail Page
- Tabbed interface (Overview, Tasks, Members)
- Overview: StatCards showing totals
- Tasks: Full Kanban board filtered to project
- Members: MemberManager component
- Back navigation to projects list

### Dashboard Home Updates
- Recent projects section (first 4)
- "View All Projects" link
- Maintains existing stats and employee status

### Employees Page Updates
- ProjectSelector dropdown added
- Filter employees by project membership
- Maintains month/year filters

### Kanban Page Updates
- ProjectSelector dropdown added
- Filter tasks by project_id
- Maintains drag-and-drop functionality

## Component Reusability

### StatCard
Props: `title`, `value`, `icon`, `color`, `iconBg`
Used in: DashboardHome, ProjectDetail

### ProjectCard
Props: `project` object
Used in: Projects page, DashboardHome (recent projects)

### ProjectSelector
Props: `value`, `onChange`, `className`
Used in: Employees, Kanban (filtering)

### MemberManager
Props: `projectId`
Handles own state, API calls, modals
Used in: ProjectDetail Members tab

## Data Flow Examples

### Creating a Project
1. User clicks "Create Project" button
2. CreateProjectModal opens with form
3. User fills name, description, color, selects members
4. Modal calls `dashboardApi.createProject(data)`
5. If members selected, calls `dashboardApi.addProjectMembers(projectId, userIds)`
6. Modal calls `onSuccess()` callback
7. Parent page refetches projects list
8. Modal closes

### Filtering by Project
1. User selects project in ProjectSelector
2. Component sets local state `selectedProject`
3. Parent filters data: `employees.filter(emp => emp.projects?.some(p => p.id === projectId))`
4. Filtered data passed to child components
5. Empty state shows "No employees in this project" message

## Performance Considerations
- No unnecessary re-renders (components only re-render on prop changes)
- API calls made once at page load, refetch on user actions
- Filter operations done in-memory (arrays are small)
- No memoization needed (simple filter operations)
- Loading states prevent multiple API calls

## Accessibility Notes
- Semantic HTML: `<button>` for actions, `<select>` for dropdowns
- Clear labels on form inputs
- Focus styles via Tailwind `focus:ring-2`
- Color contrast meets WCAG AA (checked gradients)
- Keyboard navigation works (native elements)

## UI Redesign - Clean Admin Dashboard (March 2026)

### Design Principles Applied
1. **Tables for Data** - Replaced card grids with proper HTML tables for Projects, Employees, and Members lists
2. **Minimal Rounded Corners** - Changed from `rounded-2xl`/`rounded-xl` to `rounded-md` throughout
3. **Clean Color Scheme** - Removed gradient backgrounds, use solid colors (grays + blue accent)
4. **Professional Typography** - Consistent font weights and sizes
5. **Simple Borders** - Use borders instead of shadows for structure

### Components Redesigned
- ✅ Projects page: Card grid → HTML table
- ✅ Employees page: Card grid → HTML table
- ✅ ProjectDetail: Simplified stat cards, reduced border radius
- ✅ MemberManager: List → HTML table
- ✅ StatCard: Cleaner sizing (`w-10 h-10` icons, `text-2xl` values)
- ✅ ProjectCard: Simplified for dashboard mini-cards
- ✅ All modals: `rounded-2xl` → `rounded-md`
- ✅ All avatars: Gradients → solid `bg-blue-500`, consistent `rounded-md`
- ✅ CreateProjectModal: Simplified inputs and color selector

### Standard Patterns

#### HTML Table Pattern
```jsx
<div className="bg-white rounded-md border border-gray-200 overflow-hidden">
  <table className="min-w-full divide-y divide-gray-200">
    <thead className="bg-gray-50">
      <tr>
        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
          Header
        </th>
      </tr>
    </thead>
    <tbody className="bg-white divide-y divide-gray-200">
      <tr className="hover:bg-gray-50 cursor-pointer transition-colors">
        <td className="px-6 py-4 whitespace-nowrap">
          <span className="text-sm text-gray-900">Content</span>
        </td>
      </tr>
    </tbody>
  </table>
</div>
```

#### Button Patterns
- Primary: `px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 text-sm font-medium`
- Secondary: `px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-md text-sm font-medium`

#### Avatar Pattern (10x10 for tables, 8x8 for modals)
```jsx
<div className="w-10 h-10 rounded-md bg-blue-500 flex items-center justify-center">
  <span className="text-white text-sm font-semibold">
    {name?.charAt(0).toUpperCase()}
  </span>
</div>
```

### Anti-Patterns to Avoid
- ❌ Gradient backgrounds (`bg-gradient-to-br`)
- ❌ Excessive rounded corners (`rounded-2xl`, `rounded-xl`)
- ❌ Shadow overuse (`shadow-2xl`, `shadow-lg`)
- ❌ Card-based layouts for tabular data
- ❌ Over-decorative empty states

## Future Optimization Opportunities
- Add React Query for server state caching
- Implement optimistic updates on task status changes
- Add debouncing if search/filter becomes slow
- Consider virtualization if project lists grow large (100+)
- Extract color palette to shared constants
- Table sorting and filtering for larger datasets

## Common Patterns to Follow
1. **New page**: Create in `/src/pages/`, add route in `App.jsx`, add nav link in `Layout.jsx`
2. **New component**: Create in `/src/components/`, import where needed
3. **New API call**: Add to `dashboardApi` in `api.js`
4. **Modal pattern**: Fixed overlay + centered card + header/body/footer
5. **List with add**: Show list, "Add" button opens modal, modal has checkbox list
6. **Filter pattern**: Selector component + local state + array filter

## Files Modified
- `/src/api.js` - Extended with project endpoints
- `/src/App.jsx` - Added project routes
- `/src/components/Layout.jsx` - Added Projects nav link
- `/src/pages/DashboardHome.jsx` - Added recent projects section
- `/src/pages/Employees.jsx` - Added project filter
- `/src/pages/Kanban.jsx` - Added project filter

## Files Created
- `/src/components/ProjectCard.jsx`
- `/src/components/ProjectSelector.jsx`
- `/src/components/MemberManager.jsx`
- `/src/components/TaskAssignmentPicker.jsx`
- `/src/components/CreateProjectModal.jsx`
- `/src/pages/Projects.jsx`
- `/src/pages/ProjectDetail.jsx`

## Backend API Assumptions
Assumed backend provides these endpoints:
- `GET /api/dashboard/projects` - Returns array of projects
- `POST /api/dashboard/projects` - Creates project
- `GET /api/dashboard/projects/:id` - Returns project details
- `POST /api/dashboard/projects/:id` - Updates project
- `POST /api/dashboard/projects/:id/delete` - Deletes project
- `GET /api/dashboard/projects/:id/members` - Returns members array
- `POST /api/dashboard/projects/:id/members` - Adds members
- `POST /api/dashboard/projects/:id/members/remove` - Removes member
- `GET /api/dashboard/projects/:id/tasks` - Returns tasks array
- `POST /api/dashboard/projects/:id/tasks` - Creates task in project
- `POST /api/dashboard/tasks/:id/assign-users` - Multi-user assignment
- `POST /api/dashboard/tasks/:id/unassign` - Removes user from task

Expected data shapes:
- Project: `{ id, name, description, color, member_count, task_count, total_hours }`
- Task: `{ id, name, status, max_hours, total_tracked_seconds, project_id, assignees }`
- User: `{ id, name, email, picture }`
