# GitHub Issues - Home Automation RS

Synced from GitHub Issues on 2026-01-27

## Priority Legend

| Priority | Description                                             |
| -------- | ------------------------------------------------------- |
| **P0**   | Critical - Blocking functionality, must fix immediately |
| **P1**   | High - Important feature/fix, should be done soon       |
| **P2**   | Medium - Nice to have, planned for next iteration       |
| **P3**   | Low - Enhancement, can be deferred                      |

## Cost Legend (Story Points)

| Points | Effort                     |
| ------ | -------------------------- |
| **1**  | Trivial - < 1 hour         |
| **2**  | Small - 1-4 hours          |
| **3**  | Medium - 4-8 hours (1 day) |
| **5**  | Large - 2-3 days           |
| **8**  | XL - 1 week                |
| **13** | XXL - 2+ weeks             |

---

## Summary Table

| #   | Title                                              | Priority | Cost | Category | Status    |
| --- | -------------------------------------------------- | -------- | ---- | -------- | --------- |
| 6   | Chart not reinitialized                            | P0       | 3    | Bug      | ✅ Closed |
| 14  | "Unselect All" button hidden on lower resolutions  | P1       | 2    | Bug      | ✅ Closed |
| 5   | Responsive application                             | P1       | 8    | Feature  | ✅ Closed |
| 4   | Synchronization of consumption and metrics         | P1       | 5    | Feature  | Open      |
| 10  | Improve curve visibility                           | P2       | 5    | UX       | ✅ Closed |
| 8   | Correlation between rules and metrics              | P2       | 8    | Feature  | Open      |
| 9   | Disk space monitoring                              | P2       | 3    | Feature  | Open      |
| 7   | Variables in rules                                 | P2       | 8    | Feature  | Open      |
| 16  | Standardize time display format                    | P2       | 3    | UX       | Open      |
| 15  | Simplify legend display                            | P2       | 3    | UX       | ✅ Closed |
| 12  | Display power values on consumption curve          | P2       | 3    | Feature  | Open      |
| 13  | Modify binary metrics display                      | P2       | 3    | UX       | Open      |
| 21  | Simplify metric display - remove duplicate values  | P2       | 2    | UX       | ✅ Closed |
| 22  | Reduce space used by on/off choices                | P2       | 3    | UX       | Open      |
| 23  | Preserve sensor selection when changing time range | P2       | 3    | UX       | ✅ Closed |
| 11  | Identify zoomable zones                            | P3       | 2    | UX       | ✅ Closed |
| 17  | Show All/Unselect All buttons positioning          | P3       | 1    | UX       | Open      |
| 2   | Make the 0°C line more visible                     | P3       | 1    | UX       | Open      |
| 3   | User notes                                         | P3       | 5    | Feature  | Open      |
| 24  | Add building floor plan with sensor locations      | P3       | 13   | Feature  | Open      |

**Open Issues:** 14 | **In Review:** 1 | **Closed Issues:** 5 | **Total Estimated Effort (Open):** ~60 story points

---

## Detailed Tickets

---

### TICKET-006: Chart not reinitialized when switching views ✅ CLOSED

**GitHub Issue:** [#6](https://github.com/scaraude/home-automation-rs/issues/6)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P0 - Critical        |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | Bug                  |
| **Component** | Frontend - Charts    |
| **Status**    | ✅ Closed            |

#### Description

When switching between different chart types (e.g., "sensors" and "consumption"), curves do not display correctly. Users must refresh the entire page to get the correct display. The chart component is not properly reinitialized or cleared when changing views.

---

### TICKET-014: "Unselect All" button hidden on lower resolution screens

**GitHub Issue:** [#14](https://github.com/scaraude/home-automation-rs/issues/14)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P1 - High            |
| **Cost**      | 2 points (1-4 hours) |
| **Category**  | Bug                  |
| **Component** | Frontend - UI/CSS    |
| **Status**    | ✅ Closed            |

#### Description

The "Unselect All" button disappears on screens with lower resolutions (e.g., 1440x900 on Mac). The button becomes hidden or obscured by other UI elements when the available viewport is smaller. This is a screen resolution issue, not a browser zoom issue.

#### Implementation Steps

1. [x] Identify the CSS rules affecting the button container
2. [x] Check for fixed-width containers or overflow:hidden properties
3. [x] Convert to flexbox/grid layout if using absolute positioning
4. [x] Test at various screen resolutions: 1280x720, 1440x900, 1920x1080
5. [x] Ensure proper z-index stacking and element wrapping
6. [x] Verify button remains accessible with keyboard navigation

#### Acceptance Criteria

- [x] Button visible and clickable at all screen resolutions (1280x720 and up)
- [x] Layout properly wraps or scrolls on smaller screens
- [x] Button accessible via keyboard (Tab navigation)

#### Resolution Notes

- Removed fixed graph section heights so the chart and toolbar can wrap without overlapping the sensor header.
- Clamped the chart height to responsive bounds to preserve visual balance while freeing vertical space for actions.

---

### TICKET-005: Responsive application for mobile devices ✅ CLOSED

**GitHub Issue:** [#5](https://github.com/scaraude/home-automation-rs/issues/5)

| Attribute     | Value             |
| ------------- | ----------------- |
| **Priority**  | P1 - High         |
| **Cost**      | 8 points (1 week) |
| **Category**  | Feature           |
| **Component** | Frontend - All    |
| **Status**    | ✅ Closed         |

#### Description

Make the application fully usable on smartphones. Currently, the application is not optimized for mobile devices, leading to poor user experience on smaller screens.

---

### TICKET-004: Synchronization of consumption and metrics

**GitHub Issue:** [#4](https://github.com/scaraude/home-automation-rs/issues/4)

| Attribute     | Value               |
| ------------- | ------------------- |
| **Priority**  | P1 - High           |
| **Cost**      | 5 points (2-3 days) |
| **Category**  | Feature             |
| **Component** | Frontend - Charts   |

#### Description

Add the ability to visualize electrical consumption data synchronized with temperature and humidity curves. This enables users to identify cause-and-effect relationships between energy consumption and environmental conditions.

#### Implementation Steps

1. [ ] Design synchronized chart layout (recommend stacked charts with shared X-axis)
2. [ ] Create shared time range state for chart synchronization
3. [ ] Implement synchronized zoom/pan across all stacked charts
4. [ ] Add multi-axis support if using single chart approach
5. [ ] Create toggle to enable/disable metric overlay
6. [ ] Ensure proper Y-axis scaling for different unit types (Watts vs °C vs %)
7. [ ] Add visual correlation indicators (vertical line on hover across all charts)
8. [ ] Test with real consumption + sensor data

#### Acceptance Criteria

- [ ] Can view consumption and temperature/humidity on synchronized timeline
- [ ] Zoom on one chart zooms all synchronized charts
- [ ] Hover shows values across all metrics at same timestamp
- [ ] Clear visual distinction between different metric types

---

### TICKET-010: Improve curve visibility and identification ✅ CLOSED

**GitHub Issue:** [#10](https://github.com/scaraude/home-automation-rs/issues/10)

| Attribute     | Value               |
| ------------- | ------------------- |
| **Priority**  | P2 - Medium         |
| **Cost**      | 5 points (2-3 days) |
| **Category**  | UX Enhancement      |
| **Component** | Frontend - Charts   |
| **Status**    | ✅ Closed           |

#### Description

Improve the ability to identify curves on charts. Users should be able to click on a curve and immediately identify which sensor/metric it represents. Currently difficult to distinguish between multiple overlapping curves.

---

### TICKET-008: Correlation between rules and metrics visualization

**GitHub Issue:** [#8](https://github.com/scaraude/home-automation-rs/issues/8)

| Attribute     | Value              |
| ------------- | ------------------ |
| **Priority**  | P2 - Medium        |
| **Cost**      | 8 points (1 week)  |
| **Category**  | Feature            |
| **Component** | Frontend + Backend |

#### Description

Add the ability to visualize graphically when automation rules are triggered in relation to environmental metrics. This helps users understand the impact of rule execution and cause-and-effect relationships.

#### Implementation Steps

**Backend:**

1. [ ] Create API endpoint `GET /api/automation/executions?start=&end=` for time-range queries
2. [ ] Include rule name, action, trigger time, and success status in response
3. [ ] Optimize query for chart time ranges

**Frontend:** 4. [ ] Fetch rule execution data for current chart time range 5. [ ] Render vertical markers or highlighted zones on charts at execution times 6. [ ] Use different colors/icons for different rule types 7. [ ] Add tooltips showing rule details on marker hover 8. [ ] Implement toggle to show/hide rule markers 9. [ ] Add rule execution timeline layer below charts 10. [ ] Allow clicking markers to navigate to rule details

#### Acceptance Criteria

- [ ] Rule execution markers visible on metric charts
- [ ] Tooltips show rule name, action, and timestamp
- [ ] Can filter which rules are displayed
- [ ] Markers synchronized across all chart views

---

### TICKET-009: Disk space monitoring

**GitHub Issue:** [#9](https://github.com/scaraude/home-automation-rs/issues/9)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | Feature              |
| **Component** | Backend + Frontend   |

#### Description

As data accumulates over time, monitor available disk space to prevent storage issues. Display remaining disk space on the "system logs" page.

#### Implementation Steps

**Backend:**

1. [ ] Add disk space metrics to `monitor.sh` output (df command)
2. [ ] Create API endpoint `GET /api/system/disk-usage`
3. [ ] Return total, used, free space in bytes and percentage

**Frontend:** 4. [ ] Create `DiskSpaceWidget.svelte` component 5. [ ] Display progress bar with color coding (green/yellow/red) 6. [ ] Show absolute values (e.g., "12.4 GB / 32 GB used") 7. [ ] Show percentage used 8. [ ] Add to system logs page 9. [ ] Optional: Add warning banner when space is low

#### Acceptance Criteria

- [ ] Disk usage visible on system logs page
- [ ] Color coding indicates health status
- [ ] Updates periodically (every 60 seconds)
- [ ] Warning displayed when <10% free

---

### TICKET-007: Variables in automation rules

**GitHub Issue:** [#7](https://github.com/scaraude/home-automation-rs/issues/7)

| Attribute     | Value              |
| ------------- | ------------------ |
| **Priority**  | P2 - Medium        |
| **Cost**      | 8 points (1 week)  |
| **Category**  | Feature            |
| **Component** | Backend + Frontend |

#### Description

Allow automation rule conditions to reference other sensor values instead of only fixed values. For example: "temperature > bedroom temperature" instead of "temperature > 20°C".

#### Implementation Steps

**Backend:**

1. [ ] Design condition schema extension for variable references
   ```json
   {
     "type": "sensor_comparison",
     "left_sensor_id": "living_room_temp",
     "operator": ">",
     "right_sensor_id": "bedroom_temp"
   }
   ```
2. [ ] Update `automation_rules` table or add migration
3. [ ] Modify rule evaluation engine to resolve sensor values
4. [ ] Handle cases where referenced sensor has no recent data
5. [ ] Add validation for circular references

**Frontend:** 6. [ ] Update rule condition UI to allow sensor selection 7. [ ] Add dropdown/selector for "Fixed Value" vs "Sensor Value" 8. [ ] Show available sensors based on metric type 9. [ ] Display friendly sensor names in condition preview

#### Acceptance Criteria

- [ ] Can create rules comparing two sensors
- [ ] Rule evaluation correctly uses live sensor values
- [ ] UI clearly shows which sensors are being compared
- [ ] Graceful handling when sensor data unavailable

---

### TICKET-016: Standardize time display format

**GitHub Issue:** [#16](https://github.com/scaraude/home-automation-rs/issues/16)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | UX Enhancement       |
| **Component** | Frontend             |

#### Description

Time is displayed inconsistently across charts - some show 24-hour format (18:00), others show 12-hour format (6:00 p.m.). Format should be uniform and respect user preferences.

#### Implementation Steps

1. [ ] Create centralized time formatting utility in `frontend/src/lib/utils/time.ts`
2. [ ] Detect browser locale as default (`navigator.language`)
3. [ ] Add user preference setting in UI (24h vs 12h)
4. [ ] Store preference in localStorage
5. [ ] Update all chart X-axis formatters to use utility
6. [ ] Update all tooltips to use utility
7. [ ] Update all timestamp displays (cards, lists, etc.)
8. [ ] Add settings toggle in preferences/settings area

#### Acceptance Criteria

- [ ] Consistent time format across all views
- [ ] User can choose 12h or 24h format
- [ ] Preference persists across sessions
- [ ] Defaults to browser locale setting

---

### TICKET-015: Simplify legend display ✅ CLOSED

**GitHub Issue:** [#15](https://github.com/scaraude/home-automation-rs/issues/15)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | UX Enhancement       |
| **Component** | Frontend - Charts    |
| **Status**    | ✅ Closed            |

#### Description

Chart legends are cluttered with repetitive information. Sensor names are duplicated for each metric type (e.g., "WC (Temp)" and "WC (Humidity)"). Labels are verbose and take up unnecessary space.

---

### TICKET-012: Display power values on consumption curve

**GitHub Issue:** [#12](https://github.com/scaraude/home-automation-rs/issues/12)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | Feature              |
| **Component** | Frontend - Charts    |

#### Description

The consumption chart shows power usage over time but doesn't display actual wattage values on the curve. Users must estimate from Y-axis. Show power values directly on plateau segments.

#### Implementation Steps

1. [ ] Detect plateau segments in consumption data (horizontal periods)
2. [ ] Add labels at center of plateaus showing power value (e.g., "1.2kW")
3. [ ] Implement smart label hiding for narrow plateaus
4. [ ] Use abbreviated format (kW for values >= 1000W)
5. [ ] Add toggle to show/hide value labels
6. [ ] Ensure tooltip also shows power value on hover
7. [ ] Optional: Show duration of each plateau in tooltip

#### Acceptance Criteria

- [ ] Power values visible on significant plateaus
- [ ] Labels don't overlap
- [ ] Can toggle labels on/off
- [ ] Tooltip shows value and duration

---

### TICKET-013: Modify binary metrics display

**GitHub Issue:** [#13](https://github.com/scaraude/home-automation-rs/issues/13)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | UX Enhancement       |
| **Component** | Frontend - Cards     |

#### Description

Binary (on/off) metrics use too much space and aren't visually distinct from continuous metrics. They should have a compact, distinctive visual representation.

#### Implementation Steps

1. [ ] Create compact `BinaryMetricCard.svelte` component
2. [ ] Use horizontal layout: icon + name + status badge
3. [ ] Implement clear ON/OFF visual indicators (colors, icons)
4. [ ] Group binary sensors separately from continuous sensors
5. [ ] Add state transition animation (smooth color change)
6. [ ] Reduce card height compared to standard sensor cards
7. [ ] Support grid layout for multiple binary sensors
8. [ ] Update main dashboard to use new component for binary types

#### Acceptance Criteria

- [ ] Binary sensors visually distinct from continuous sensors
- [ ] Compact horizontal layout saves space
- [ ] ON/OFF state immediately recognizable
- [ ] Smooth animations on state change

---

### TICKET-021: Simplify metric display - remove duplicate values 🔄 PR #25

**GitHub Issue:** [#21](https://github.com/scaraude/home-automation-rs/issues/21)
**Pull Request:** [#25](https://github.com/scaraude/home-automation-rs/pull/25)

| Attribute     | Value                 |
| ------------- | --------------------- |
| **Priority**  | P2 - Medium           |
| **Cost**      | 2 points (1-4 hours)  |
| **Category**  | UX Enhancement        |
| **Component** | Backend - MQTT        |
| **Status**    | 🔄 In Review (PR #25) |

#### Description

The metric display on the chart shows values redundantly. When hovering over or viewing a curve, the same metric value appears twice in the interface, creating unnecessary visual clutter and confusion.

#### Implementation (Completed)

Added a deduplication filter in the MQTT layer to prevent duplicate sensor readings from being stored:

- New `src/mqtt/dedup_filter.rs` module with configurable time window
- Filters identical readings (same device, temperature, humidity) within the window
- Reduces visual clutter in charts by eliminating redundant data points

#### Files Changed

- `src/mqtt/dedup_filter.rs` (new) - Deduplication filter implementation
- `src/mqtt/event_loop.rs` - Integration with MQTT message handling
- `src/mqtt/mod.rs` - Module export

#### Acceptance Criteria

- [x] Duplicate readings filtered out when received within the time window
- [x] Unique readings still stored normally
- [x] Chart displays show clean data without redundant points

---

### TICKET-022: Reduce space used by on/off choices (NEW)

**GitHub Issue:** [#22](https://github.com/scaraude/home-automation-rs/issues/22)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | UX Enhancement       |
| **Component** | Frontend - UI        |

#### Description

Some on/off toggle controls take up too much screen space, reducing the available area for displaying actual data and creating visual clutter.

#### Implementation Steps

1. [ ] Replace large switches with smaller, more compact toggle controls
2. [ ] Use minimal padding and margins around toggle elements
3. [ ] Implement icon-based toggles (✓/✗, ●/○) instead of full switch controls
4. [ ] Place labels inline with toggles instead of above/below
5. [ ] Provide different layouts for desktop (compact) vs mobile (touch-friendly)

#### Acceptance Criteria

- [ ] Toggle controls use significantly less space
- [ ] Toggles remain touch-friendly on mobile (minimum 44x44px)
- [ ] Functionality preserved with improved visual efficiency

---

### TICKET-023: Preserve sensor selection when changing time range (NEW)

**GitHub Issue:** [#23](https://github.com/scaraude/home-automation-rs/issues/23)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P2 - Medium          |
| **Cost**      | 3 points (4-8 hours) |
| **Category**  | UX Enhancement       |
| **Component** | Frontend - Charts    |

#### Description

When changing the time range or duration (e.g., from 24 hours to 1 week), all sensors/metrics are automatically selected, overriding the user's previous selection. This forces users to manually deselect unwanted sensors again.

#### Implementation Steps

1. [x] Store the current sensor/metric selection state in memory or local storage
2. [x] Maintain the same selection when user changes time range
3. [x] Track selected sensors independently from time range settings
4. [x] Handle scenarios where sensors were added/removed between time range changes
5. [x] Provide clear visual feedback that selection has been preserved

#### Acceptance Criteria

- [x] Selection persists when changing time range
- [x] Works across all time range options (24h, 1 week, 1 month, etc.)
- [x] Graceful handling when selected sensor has no data in new time range

---

### TICKET-011: Identify zoomable zones ✅ CLOSED

**GitHub Issue:** [#11](https://github.com/scaraude/home-automation-rs/issues/11)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P3 - Low             |
| **Cost**      | 2 points (1-4 hours) |
| **Category**  | UX Enhancement       |
| **Component** | Frontend - Charts    |
| **Status**    | ✅ Closed            |

#### Description

Some areas are zoomable via mouse wheel, others are not. Users don't know which zones support zoom. Add visual indicators to distinguish zoomable areas.

---

### TICKET-017: Show All/Unselect All buttons positioning

**GitHub Issue:** [#17](https://github.com/scaraude/home-automation-rs/issues/17)

| Attribute     | Value              |
| ------------- | ------------------ |
| **Priority**  | P3 - Low           |
| **Cost**      | 1 point (< 1 hour) |
| **Category**  | UX Enhancement     |
| **Component** | Frontend - UI      |

#### Description

The "Show All Sensors" and "Unselect All" buttons are positioned far apart. These complementary functions should be grouped together.

#### Implementation Steps

1. [ ] Move both buttons to same toolbar area
2. [ ] Position buttons side-by-side
3. [ ] Use consistent button styling
4. [ ] Consider combining into single toggle button
5. [ ] Optional: Add icons (✓ for select all, ✗ for unselect)

#### Acceptance Criteria

- [ ] Both buttons in same visual group
- [ ] Consistent styling
- [ ] Reduced mouse travel between actions

---

### TICKET-002: Make the 0°C line more visible

**GitHub Issue:** [#2](https://github.com/scaraude/home-automation-rs/issues/2)

| Attribute     | Value              |
| ------------- | ------------------ |
| **Priority**  | P3 - Low           |
| **Cost**      | 1 point (< 1 hour) |
| **Category**  | UX Enhancement     |
| **Component** | Frontend - Charts  |

#### Description

The 0°C line on temperature charts should be more prominent to better visualize when temperatures reach freezing point.

#### Implementation Steps

1. [ ] Identify chart configuration for grid lines
2. [ ] Add special styling for Y=0 line
3. [ ] Set line color (suggest blue or red)
4. [ ] Increase line thickness (2-3px)
5. [ ] Optional: Add dashed style
6. [ ] Test with data that spans above and below 0°C

#### Acceptance Criteria

- [ ] 0°C line clearly visible and distinct from other grid lines
- [ ] Visually indicates freezing threshold

---

### TICKET-003: User notes annotation system

**GitHub Issue:** [#3](https://github.com/scaraude/home-automation-rs/issues/3)

| Attribute     | Value               |
| ------------- | ------------------- |
| **Priority**  | P3 - Low            |
| **Cost**      | 5 points (2-3 days) |
| **Category**  | Feature             |
| **Component** | Backend + Frontend  |

#### Description

Add a feature allowing users to annotate data with notes about events that may impact metrics (heating changes, insulation work, sensor relocation, etc.).

#### Implementation Steps

**Backend:**

1. [ ] Create database migration for `user_notes` table
   ```sql
   CREATE TABLE user_notes (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     author TEXT,
     timestamp INTEGER NOT NULL,
     description TEXT NOT NULL,
     created_at INTEGER NOT NULL
   );
   ```
2. [ ] Create CRUD API endpoints:
   - `GET /api/notes?start=&end=`
   - `POST /api/notes`
   - `PUT /api/notes/{id}`
   - `DELETE /api/notes/{id}`

**Frontend:** 3. [ ] Create "Add Note" button/dialog 4. [ ] Build note creation form (date picker, description field) 5. [ ] Display note markers on charts as vertical lines/icons 6. [ ] Show note content in tooltip on hover 7. [ ] Create notes list view for browsing/editing

#### Acceptance Criteria

- [ ] Can create notes with timestamp and description
- [ ] Notes visible as markers on metric charts
- [ ] Can edit and delete existing notes
- [ ] Notes persist across sessions

---

### TICKET-024: Add building floor plan with sensor locations (NEW)

**GitHub Issue:** [#24](https://github.com/scaraude/home-automation-rs/issues/24)

| Attribute     | Value                |
| ------------- | -------------------- |
| **Priority**  | P3 - Low             |
| **Cost**      | 13 points (2+ weeks) |
| **Category**  | Feature              |
| **Component** | Frontend + Backend   |

#### Description

Add a visual floor plan or building map that displays the physical location of each sensor/detector. This would provide users with spatial context for their sensor data.

#### Implementation Steps

**Backend:**

1. [ ] Create database table for floor plans and sensor positions
2. [ ] Create API endpoints for floor plan upload and management
3. [ ] Store sensor positions relative to floor plan dimensions

**Frontend:** 4. [ ] Allow users to upload a building floor plan image (PNG, JPG, SVG) 5. [ ] Provide tools to mark and position sensors on the floor plan 6. [ ] Display sensors as icons/markers with color coding for current status 7. [ ] Show current values in tooltips when hovering over sensor markers 8. [ ] Link floor plan view with chart views (click sensor on map → show its chart) 9. [ ] Support multiple floors/levels if needed 10. [ ] Add zoom and pan capabilities for large floor plans

#### Acceptance Criteria

- [ ] Can upload and display a floor plan image
- [ ] Can position sensors on the floor plan
- [ ] Sensors show current status/values
- [ ] Click on sensor marker opens detailed view/chart
- [ ] Works for buildings with multiple floors

---

## Recommended Implementation Order

### Phase 1: Bug Fixes (Current Priority)

1. **TICKET-014** - Unselect All button visibility (P1) - 2 pts ✅ Closed

### Phase 2: Core Features

2. **TICKET-004** - Sync consumption & metrics (P1) - 5 pts

### Phase 3: UX Improvements

3. **TICKET-016** - Time format standardization (P2) - 3 pts
4. **TICKET-021** - Remove duplicate values (P2) - 2 pts
5. **TICKET-023** - Preserve sensor selection (P2) - 3 pts
6. **TICKET-022** - Compact on/off toggles (P2) - 3 pts

### Phase 4: Advanced Features

7. **TICKET-008** - Rules & metrics correlation (P2) - 8 pts
8. **TICKET-007** - Variables in rules (P2) - 8 pts

### Phase 5: Polish & Enhancements

9. **TICKET-009** - Disk monitoring (P2) - 3 pts
10. **TICKET-012** - Power values display (P2) - 3 pts
11. **TICKET-013** - Binary metrics display (P2) - 3 pts

### Phase 6: Low Priority (As time permits)

12. **TICKET-017** - Button positioning (P3) - 1 pt
13. **TICKET-002** - 0°C line visibility (P3) - 1 pt
14. **TICKET-003** - User notes (P3) - 5 pts
15. **TICKET-024** - Floor plan visualization (P3) - 13 pts

---

## Notes

- All time estimates assume familiarity with the codebase
- Some tickets may have dependencies not explicitly listed
- Frontend tickets require testing on Pi dashboard (slow network)
- Backend changes require `make quick-deploy` for testing on Pi
- Closed tickets (#5, #6, #10, #11, #15) have been completed
