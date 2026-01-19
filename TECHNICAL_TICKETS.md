# Technical Tickets - Home Automation RS

Generated from GitHub Issues on 2026-01-20

## Priority Legend

| Priority | Description |
|----------|-------------|
| **P0** | Critical - Blocking functionality, must fix immediately |
| **P1** | High - Important feature/fix, should be done soon |
| **P2** | Medium - Nice to have, planned for next iteration |
| **P3** | Low - Enhancement, can be deferred |

## Cost Legend (Story Points)

| Points | Effort |
|--------|--------|
| **1** | Trivial - < 1 hour |
| **2** | Small - 1-4 hours |
| **3** | Medium - 4-8 hours (1 day) |
| **5** | Large - 2-3 days |
| **8** | XL - 1 week |
| **13** | XXL - 2+ weeks |

---

## Summary Table

| # | Title | Priority | Cost | Category |
|---|-------|----------|------|----------|
| 6 | Chart not reinitialized | P0 | 3 | Bug |
| 14 | "Unselect All" button hidden at 100% zoom | P1 | 2 | Bug |
| 5 | Responsive application | P1 | 8 | Feature |
| 4 | Synchronization of consumption and metrics | P1 | 5 | Feature |
| 10 | Improve curve visibility | P2 | 5 | UX |
| 8 | Correlation between rules and metrics | P2 | 8 | Feature |
| 9 | Disk space monitoring | P2 | 3 | Feature |
| 7 | Variables in rules | P2 | 8 | Feature |
| 16 | Standardize time display format | P2 | 3 | UX |
| 15 | Simplify legend display | P2 | 3 | UX |
| 12 | Display power values on consumption curve | P2 | 3 | Feature |
| 13 | Modify binary metrics display | P2 | 3 | UX |
| 11 | Identify zoomable zones | P3 | 2 | UX |
| 17 | Show All/Unselect All buttons positioning | P3 | 1 | UX |
| 2 | Make the 0°C line more visible | P3 | 1 | UX |
| 3 | User notes | P3 | 5 | Feature |

**Total Estimated Effort:** 63 story points

---

## Detailed Tickets

---

### TICKET-006: Chart not reinitialized when switching views

**GitHub Issue:** [#6](https://github.com/scaraude/home-assistant-rs/issues/6)

| Attribute | Value |
|-----------|-------|
| **Priority** | P0 - Critical |
| **Cost** | 3 points (4-8 hours) |
| **Category** | Bug |
| **Component** | Frontend - Charts |

#### Description
When switching between different chart types (e.g., "sensors" and "consumption"), curves do not display correctly. Users must refresh the entire page to get the correct display. The chart component is not properly reinitialized or cleared when changing views.

#### Technical Analysis
- **Root Cause:** Likely the chart library instance (Chart.js or similar) is not being destroyed/recreated when the view changes
- **Affected Files:** `frontend/src/lib/GraphModal.svelte`, chart-related components
- **Impact:** Major usability issue - users cannot switch views without page refresh

#### Implementation Steps
1. [ ] Identify the chart library being used and its lifecycle management
2. [ ] Review current component mounting/unmounting logic in Svelte
3. [ ] Implement proper chart destruction on component unmount (`onDestroy`)
4. [ ] Clear chart data and canvas before reinitializing with new data
5. [ ] Add reactive statement to watch for data type changes
6. [ ] Test switching between all chart views (sensors, consumption, etc.)
7. [ ] Verify no memory leaks from orphaned chart instances

#### Acceptance Criteria
- [ ] Charts display correctly when switching between views without page refresh
- [ ] No console errors during view transitions
- [ ] Chart animations work properly on reinitialization
- [ ] Memory usage remains stable after multiple view switches

---

### TICKET-014: "Unselect All" button hidden at 100% zoom or higher

**GitHub Issue:** [#14](https://github.com/scaraude/home-assistant-rs/issues/14)

| Attribute | Value |
|-----------|-------|
| **Priority** | P1 - High |
| **Cost** | 2 points (1-4 hours) |
| **Category** | Bug |
| **Component** | Frontend - UI/CSS |

#### Description
The "Unselect All" button is only visible when the browser zoom level is set to 90% or less. At 100% zoom and above, the button becomes hidden or obscured by other UI elements, making the functionality inaccessible.

#### Technical Analysis
- **Root Cause:** Fixed positioning, overflow:hidden, or absolute positioning issues
- **Affected Files:** `frontend/src/lib/GraphModal.svelte` or related chart component CSS
- **Impact:** Accessibility issue - users at standard zoom cannot access functionality

#### Implementation Steps
1. [ ] Identify the CSS rules affecting the button container
2. [ ] Check for fixed-width containers or overflow:hidden properties
3. [ ] Convert to flexbox/grid layout if using absolute positioning
4. [ ] Test at zoom levels: 100%, 110%, 125%, 150%, 200%
5. [ ] Ensure proper z-index stacking
6. [ ] Verify button remains accessible with keyboard navigation

#### Acceptance Criteria
- [ ] Button visible and clickable at all zoom levels (100%-200%)
- [ ] Layout doesn't break on different screen sizes
- [ ] Button accessible via keyboard (Tab navigation)

---

### TICKET-005: Responsive application for mobile devices

**GitHub Issue:** [#5](https://github.com/scaraude/home-assistant-rs/issues/5)

| Attribute | Value |
|-----------|-------|
| **Priority** | P1 - High |
| **Cost** | 8 points (1 week) |
| **Category** | Feature |
| **Component** | Frontend - All |

#### Description
Make the application fully usable on smartphones. Currently, the application is not optimized for mobile devices, leading to poor user experience on smaller screens.

#### Technical Analysis
- **Scope:** Full frontend redesign for responsive breakpoints
- **Affected Files:** All Svelte components, CSS files
- **Dependencies:** May need mobile-specific chart configurations

#### Implementation Steps
1. [ ] Audit current layout at mobile breakpoints (320px, 375px, 414px)
2. [ ] Define responsive breakpoints (mobile: <768px, tablet: 768-1024px, desktop: >1024px)
3. [ ] Implement mobile-first CSS with media queries
4. [ ] Redesign navigation for mobile (hamburger menu or bottom nav)
5. [ ] Optimize chart components for touch interaction
   - [ ] Increase touch targets
   - [ ] Add pinch-to-zoom support
   - [ ] Simplify legends on mobile
6. [ ] Adjust card layouts (single column on mobile)
7. [ ] Optimize modal/popup sizing for small screens
8. [ ] Test on actual devices (iOS Safari, Android Chrome)
9. [ ] Implement viewport meta tag if missing

#### Acceptance Criteria
- [ ] All features accessible on mobile devices
- [ ] Charts readable and interactive on 375px width
- [ ] Touch interactions work smoothly
- [ ] No horizontal scrolling required
- [ ] Text readable without zooming

---

### TICKET-004: Synchronization of consumption and metrics

**GitHub Issue:** [#4](https://github.com/scaraude/home-assistant-rs/issues/4)

| Attribute | Value |
|-----------|-------|
| **Priority** | P1 - High |
| **Cost** | 5 points (2-3 days) |
| **Category** | Feature |
| **Component** | Frontend - Charts |

#### Description
Add the ability to visualize electrical consumption data synchronized with temperature and humidity curves. This enables users to identify cause-and-effect relationships between energy consumption and environmental conditions.

#### Technical Analysis
- **Chart Options:** Multiple Y-axes on single chart OR stacked charts with synced X-axis
- **Data Requirements:** Consumption data must be timestamped and aligned with sensor data
- **Affected Files:** Chart components, API data fetching logic

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

### TICKET-010: Improve curve visibility and identification

**GitHub Issue:** [#10](https://github.com/scaraude/home-assistant-rs/issues/10)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 5 points (2-3 days) |
| **Category** | UX Enhancement |
| **Component** | Frontend - Charts |

#### Description
Improve the ability to identify curves on charts. Users should be able to click on a curve and immediately identify which sensor/metric it represents. Currently difficult to distinguish between multiple overlapping curves.

#### Technical Analysis
- **Features Needed:** Click-to-highlight, interactive legend, distinct visual styles
- **Affected Files:** Chart components, legend components
- **Accessibility:** Must support colorblind-friendly palettes

#### Implementation Steps
1. [ ] Implement click-to-highlight functionality on curves
2. [ ] Show popup/tooltip with sensor details on curve click
3. [ ] Dim non-selected curves when one is highlighted
4. [ ] Make legend items interactive (click to toggle, hover to highlight)
5. [ ] Use distinct line styles (solid, dashed, dotted) in addition to colors
6. [ ] Implement colorblind-friendly color palette
7. [ ] Add crosshair that shows values for all curves at a time point
8. [ ] Display current value next to legend items
9. [ ] Add keyboard navigation support for accessibility

#### Acceptance Criteria
- [ ] Clicking a curve highlights it and shows details
- [ ] Legend items are interactive (click/hover)
- [ ] Curves distinguishable without relying solely on color
- [ ] Crosshair shows all values at hovered time point

---

### TICKET-008: Correlation between rules and metrics visualization

**GitHub Issue:** [#8](https://github.com/scaraude/home-assistant-rs/issues/8)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 8 points (1 week) |
| **Category** | Feature |
| **Component** | Frontend + Backend |

#### Description
Add the ability to visualize graphically when automation rules are triggered in relation to environmental metrics. This helps users understand the impact of rule execution and cause-and-effect relationships.

#### Technical Analysis
- **Backend Changes:** API endpoint for rule execution history with timestamps
- **Frontend Changes:** Overlay markers on metric charts
- **Data Source:** `automation_execution_log` table already exists

#### Implementation Steps

**Backend:**
1. [ ] Create API endpoint `GET /api/automation/executions?start=&end=` for time-range queries
2. [ ] Include rule name, action, trigger time, and success status in response
3. [ ] Optimize query for chart time ranges

**Frontend:**
4. [ ] Fetch rule execution data for current chart time range
5. [ ] Render vertical markers or highlighted zones on charts at execution times
6. [ ] Use different colors/icons for different rule types
7. [ ] Add tooltips showing rule details on marker hover
8. [ ] Implement toggle to show/hide rule markers
9. [ ] Add rule execution timeline layer below charts
10. [ ] Allow clicking markers to navigate to rule details

#### Acceptance Criteria
- [ ] Rule execution markers visible on metric charts
- [ ] Tooltips show rule name, action, and timestamp
- [ ] Can filter which rules are displayed
- [ ] Markers synchronized across all chart views

---

### TICKET-009: Disk space monitoring

**GitHub Issue:** [#9](https://github.com/scaraude/home-assistant-rs/issues/9)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 3 points (4-8 hours) |
| **Category** | Feature |
| **Component** | Backend + Frontend |

#### Description
As data accumulates over time, monitor available disk space to prevent storage issues. Display remaining disk space on the "system logs" page.

#### Technical Analysis
- **Backend:** Use system calls to get disk usage (already have `monitor.sh`)
- **Frontend:** New widget on system logs page
- **Alert Levels:** Green (>20%), Yellow (10-20%), Red (<10%)

#### Implementation Steps

**Backend:**
1. [ ] Add disk space metrics to `monitor.sh` output (df command)
2. [ ] Create API endpoint `GET /api/system/disk-usage`
3. [ ] Return total, used, free space in bytes and percentage

**Frontend:**
4. [ ] Create `DiskSpaceWidget.svelte` component
5. [ ] Display progress bar with color coding (green/yellow/red)
6. [ ] Show absolute values (e.g., "12.4 GB / 32 GB used")
7. [ ] Show percentage used
8. [ ] Add to system logs page
9. [ ] Optional: Add warning banner when space is low

#### Acceptance Criteria
- [ ] Disk usage visible on system logs page
- [ ] Color coding indicates health status
- [ ] Updates periodically (every 60 seconds)
- [ ] Warning displayed when <10% free

---

### TICKET-007: Variables in automation rules

**GitHub Issue:** [#7](https://github.com/scaraude/home-assistant-rs/issues/7)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 8 points (1 week) |
| **Category** | Feature |
| **Component** | Backend + Frontend |

#### Description
Allow automation rule conditions to reference other sensor values instead of only fixed values. For example: "temperature > bedroom temperature" instead of "temperature > 20°C".

#### Technical Analysis
- **Database Changes:** Condition schema needs to support sensor references
- **Backend Changes:** Rule evaluation must resolve sensor values at runtime
- **Frontend Changes:** UI for selecting sensors as condition values

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

**Frontend:**
6. [ ] Update rule condition UI to allow sensor selection
7. [ ] Add dropdown/selector for "Fixed Value" vs "Sensor Value"
8. [ ] Show available sensors based on metric type
9. [ ] Display friendly sensor names in condition preview

#### Acceptance Criteria
- [ ] Can create rules comparing two sensors
- [ ] Rule evaluation correctly uses live sensor values
- [ ] UI clearly shows which sensors are being compared
- [ ] Graceful handling when sensor data unavailable

---

### TICKET-016: Standardize time display format

**GitHub Issue:** [#16](https://github.com/scaraude/home-assistant-rs/issues/16)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 3 points (4-8 hours) |
| **Category** | UX Enhancement |
| **Component** | Frontend |

#### Description
Time is displayed inconsistently across charts - some show 24-hour format (18:00), others show 12-hour format (6:00 p.m.). Format should be uniform and respect user preferences.

#### Technical Analysis
- **Solution:** Centralized time formatting utility
- **Affected Files:** All components displaying timestamps
- **Storage:** User preference in localStorage

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

### TICKET-015: Simplify legend display

**GitHub Issue:** [#15](https://github.com/scaraude/home-assistant-rs/issues/15)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 3 points (4-8 hours) |
| **Category** | UX Enhancement |
| **Component** | Frontend - Charts |

#### Description
Chart legends are cluttered with repetitive information. Sensor names are duplicated for each metric type (e.g., "WC (Temp)" and "WC (Humidity)"). Labels are verbose and take up unnecessary space.

#### Technical Analysis
- **Solution:** Group metrics by sensor, use visual indicators instead of text
- **Affected Files:** Chart legend components
- **Design:** Use icons/line styles instead of "(Temp)" / "(Humidity)" text

#### Implementation Steps
1. [ ] Design new legend format (grouped by sensor)
2. [ ] Use line styles to distinguish metrics (solid = temp, dashed = humidity)
3. [ ] Add icons: 🌡️ for temperature, 💧 for humidity
4. [ ] Show sensor name once with sub-indicators
5. [ ] Reduce legend font size slightly
6. [ ] Make legend collapsible/hideable
7. [ ] Implement in all chart components
8. [ ] Update chart documentation

#### Acceptance Criteria
- [ ] Sensor names appear only once in legend
- [ ] Metrics distinguished by visual style, not just text
- [ ] Legend takes less vertical space
- [ ] Legend can be collapsed

---

### TICKET-012: Display power values on consumption curve

**GitHub Issue:** [#12](https://github.com/scaraude/home-assistant-rs/issues/12)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 3 points (4-8 hours) |
| **Category** | Feature |
| **Component** | Frontend - Charts |

#### Description
The consumption chart shows power usage over time but doesn't display actual wattage values on the curve. Users must estimate from Y-axis. Show power values directly on plateau segments.

#### Technical Analysis
- **Solution:** Add data labels on horizontal segments (plateaus)
- **Challenge:** Avoid label overlap on short segments
- **Affected Files:** Consumption chart component

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

**GitHub Issue:** [#13](https://github.com/scaraude/home-assistant-rs/issues/13)

| Attribute | Value |
|-----------|-------|
| **Priority** | P2 - Medium |
| **Cost** | 3 points (4-8 hours) |
| **Category** | UX Enhancement |
| **Component** | Frontend - Cards |

#### Description
Binary (on/off) metrics use too much space and aren't visually distinct from continuous metrics. They should have a compact, distinctive visual representation.

#### Technical Analysis
- **Solution:** Redesign binary sensor cards with toggle/badge style
- **Affected Files:** `SensorCard.svelte` or create `BinaryCard.svelte`
- **Layout:** Horizontal single-line vs current card layout

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

### TICKET-011: Identify zoomable zones

**GitHub Issue:** [#11](https://github.com/scaraude/home-assistant-rs/issues/11)

| Attribute | Value |
|-----------|-------|
| **Priority** | P3 - Low |
| **Cost** | 2 points (1-4 hours) |
| **Category** | UX Enhancement |
| **Component** | Frontend - Charts |

#### Description
Some areas are zoomable via mouse wheel, others are not. Users don't know which zones support zoom. Add visual indicators to distinguish zoomable areas.

#### Technical Analysis
- **Solution:** Cursor change, tooltips, visual cues on hover
- **Affected Files:** Chart wrapper components
- **Scope:** Relatively small CSS/hover changes

#### Implementation Steps
1. [ ] Change cursor to zoom-in icon on zoomable areas
2. [ ] Add subtle border highlight on hover for zoomable zones
3. [ ] Show "Scroll to zoom" tooltip on first hover
4. [ ] Display current zoom level while actively zooming
5. [ ] Optional: Add zoom +/- buttons as alternative to scroll

#### Acceptance Criteria
- [ ] Cursor changes over zoomable areas
- [ ] Hover reveals tooltip explaining zoom
- [ ] Consistent behavior across all charts

---

### TICKET-017: Show All/Unselect All buttons positioning

**GitHub Issue:** [#17](https://github.com/scaraude/home-assistant-rs/issues/17)

| Attribute | Value |
|-----------|-------|
| **Priority** | P3 - Low |
| **Cost** | 1 point (< 1 hour) |
| **Category** | UX Enhancement |
| **Component** | Frontend - UI |

#### Description
The "Show All Sensors" and "Unselect All" buttons are positioned far apart. These complementary functions should be grouped together.

#### Technical Analysis
- **Solution:** Move buttons to same toolbar/location
- **Affected Files:** Chart/legend component CSS
- **Scope:** Simple CSS repositioning

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

**GitHub Issue:** [#2](https://github.com/scaraude/home-assistant-rs/issues/2)

| Attribute | Value |
|-----------|-------|
| **Priority** | P3 - Low |
| **Cost** | 1 point (< 1 hour) |
| **Category** | UX Enhancement |
| **Component** | Frontend - Charts |

#### Description
The 0°C line on temperature charts should be more prominent to better visualize when temperatures reach freezing point.

#### Technical Analysis
- **Solution:** Style the zero line with distinct color/thickness
- **Affected Files:** Temperature chart configuration
- **Scope:** Single CSS/chart config change

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

**GitHub Issue:** [#3](https://github.com/scaraude/home-assistant-rs/issues/3)

| Attribute | Value |
|-----------|-------|
| **Priority** | P3 - Low |
| **Cost** | 5 points (2-3 days) |
| **Category** | Feature |
| **Component** | Backend + Frontend |

#### Description
Add a feature allowing users to annotate data with notes about events that may impact metrics (heating changes, insulation work, sensor relocation, etc.).

#### Technical Analysis
- **Database Changes:** New `user_notes` table
- **Backend Changes:** CRUD API for notes
- **Frontend Changes:** Note creation UI, markers on charts

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

**Frontend:**
3. [ ] Create "Add Note" button/dialog
4. [ ] Build note creation form (date picker, description field)
5. [ ] Display note markers on charts as vertical lines/icons
6. [ ] Show note content in tooltip on hover
7. [ ] Create notes list view for browsing/editing

#### Acceptance Criteria
- [ ] Can create notes with timestamp and description
- [ ] Notes visible as markers on metric charts
- [ ] Can edit and delete existing notes
- [ ] Notes persist across sessions

---

## Recommended Implementation Order

### Phase 1: Bug Fixes & Stability (Week 1)
1. **TICKET-006** - Chart not reinitialized (P0) - 3 pts
2. **TICKET-014** - Unselect All button visibility (P1) - 2 pts

### Phase 2: Core Features (Weeks 2-3)
3. **TICKET-005** - Responsive design (P1) - 8 pts
4. **TICKET-004** - Sync consumption & metrics (P1) - 5 pts

### Phase 3: UX Improvements (Week 4)
5. **TICKET-016** - Time format standardization (P2) - 3 pts
6. **TICKET-015** - Legend simplification (P2) - 3 pts
7. **TICKET-010** - Curve visibility (P2) - 5 pts

### Phase 4: Advanced Features (Weeks 5-6)
8. **TICKET-008** - Rules & metrics correlation (P2) - 8 pts
9. **TICKET-007** - Variables in rules (P2) - 8 pts

### Phase 5: Polish & Enhancements (Weeks 7-8)
10. **TICKET-009** - Disk monitoring (P2) - 3 pts
11. **TICKET-012** - Power values display (P2) - 3 pts
12. **TICKET-013** - Binary metrics display (P2) - 3 pts

### Phase 6: Low Priority (As time permits)
13. **TICKET-017** - Button positioning (P3) - 1 pt
14. **TICKET-002** - 0°C line visibility (P3) - 1 pt
15. **TICKET-011** - Zoomable zone indicators (P3) - 2 pts
16. **TICKET-003** - User notes (P3) - 5 pts

---

## Notes

- All time estimates assume familiarity with the codebase
- Some tickets may have dependencies not explicitly listed
- Frontend tickets require testing on Pi dashboard (slow network)
- Backend changes require `make quick-deploy` for testing on Pi
