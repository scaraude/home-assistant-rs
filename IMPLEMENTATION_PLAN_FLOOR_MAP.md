# Plan d'implémentation : Floor Map View

> **Feature**: Vue plan de maison avec devices positionnables sur un SVG de fond
> **Epic**: Visualisation spatiale des devices domotiques

---

## Vue d'ensemble

Cette feature transforme la page Network existante en une vue "Floor Map" où les devices sont représentés par des cartes compactes positionnées sur un plan SVG de la maison. Elle réutilise SvelteFlow déjà en place.

---

## Architecture technique

### Composants impactés

| Couche       | Fichiers existants                           | Modifications                  |
| ------------ | -------------------------------------------- | ------------------------------ |
| **Frontend** | `NetworkGraph.svelte`, `DeviceNode.svelte`   | Refactoring majeur             |
| **Frontend** | `Badge.svelte` (Design System)               | Réutilisation (variant `mini`) |
| **Backend**  | `db/schema.rs`, `db/queries/device_state.rs` | Nouvelle table positions       |
| **API**      | `http/routes/`                               | Nouveaux endpoints positions   |
| **Stores**   | `networkTopology.ts`                         | Migration localStorage → DB    |

### Nouveau modèle de données

```sql
-- Table pour persister les positions des devices sur le plan
CREATE TABLE device_positions (
    device_id TEXT PRIMARY KEY,
    x REAL NOT NULL,
    y REAL NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);

-- Table pour stocker le SVG du plan
CREATE TABLE floor_plan (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Singleton
    svg_content TEXT NOT NULL,
    uploaded_at INTEGER NOT NULL
);
```

---

## Tickets techniques

### Phase 1 : Infrastructure Backend (P0)

#### Ticket #FM-001 : Schema DB pour positions devices

**Priorité**: P0 | **Story Points**: 2 | **Dépendances**: Aucune

**Description**:
Créer la table `device_positions` pour persister les coordonnées x/y des devices sur le plan.

**Critères d'acceptation**:

- [x] Table `device_positions` créée avec migration automatique
- [x] Index sur `device_id` pour requêtes rapides
- [x] Foreign key avec cascade delete vers `devices`
- [x] Tests unitaires CRUD (11 tests)

**Fichiers modifiés**:

- `src/db/schema.rs` - Ajout `create_device_positions_table()`
- `src/db/queries/mod.rs` - Export nouveau module
- `src/db/queries/device_position.rs` - Nouveau fichier CRUD
- `src/models/device.rs` - Ajout model `DevicePosition`
- `src/models/mod.rs` - Export `DevicePosition`
- `src/db/tests.rs` - 11 tests unitaires

**Status**: ✅ COMPLETED

---

#### Ticket #FM-002 : Schema DB pour floor plan SVG

**Priorité**: P0 | **Story Points**: 2 | **Dépendances**: Aucune

**Description**:
Créer la table singleton `floor_plan` pour stocker le SVG uploadé.

**Critères d'acceptation**:

- [x] Table `floor_plan` créée (contrainte singleton)
- [x] Support BLOB ou TEXT pour SVG
- [x] Tests unitaires get/upsert (8 tests)

**Fichiers modifiés**:

- `src/db/schema.rs` - Ajout `create_floor_plan_table()`
- `src/db/queries/floor_plan.rs` - Nouveau fichier CRUD
- `src/db/queries/mod.rs` - Export nouveau module
- `src/models/device.rs` - Ajout model `FloorPlan`
- `src/models/mod.rs` - Export `FloorPlan`
- `src/db/tests.rs` - 8 tests unitaires

**Status**: ✅ COMPLETED

---

#### Ticket #FM-003 : API endpoints positions

**Priorité**: P0 | **Story Points**: 3 | **Dépendances**: FM-001

**Description**:
Exposer les endpoints REST pour gérer les positions des devices.

**Endpoints**:

```
GET  /api/devices/positions          # Toutes les positions
PUT  /api/devices/{id}/position      # Update position unique
```

**Critères d'acceptation**:

- [x] Endpoints fonctionnels avec validation
- [x] Réponse JSON cohérente avec l'API existante
- [x] Event WebSocket `DevicePositionUpdated` émis

**Fichiers modifiés**:

- `src/http/routes/positions.rs` - Nouveau fichier avec handlers
- `src/http/routes/mod.rs` - Export des nouveaux handlers
- `src/http/mod.rs` - Routes ajoutées au router
- `src/events/mod.rs` - Nouveau variant `SystemEvent::DevicePositionUpdated`

**Status**: ✅ COMPLETED

---

#### Ticket #FM-004 : API endpoints floor plan

**Priorité**: P0 | **Story Points**: 3 | **Dépendances**: FM-002

**Description**:
Exposer les endpoints REST pour upload/get du plan SVG.

**Endpoints**:

```
GET  /api/floor-plan                 # Récupérer le SVG
POST /api/floor-plan                 # Upload nouveau SVG
DELETE /api/floor-plan               # Supprimer le plan
```

**Critères d'acceptation**:

- [x] Upload JSON avec svg_content (base64 ou raw SVG)
- [x] Validation SVG basique (taille max 5MB, format)
- [x] Content-Type approprié en réponse (JSON)

**Fichiers modifiés**:

- `src/http/routes/floor_plan.rs` - Nouveau fichier avec handlers GET/POST/DELETE
- `src/http/routes/mod.rs` - Export des nouveaux handlers
- `src/http/mod.rs` - Routes ajoutées au router

**Status**: ✅ COMPLETED

---

### Phase 2 : Refactoring Frontend (P0)

#### Ticket #FM-005 : Composant FloorMapCard générique

**Priorité**: P0 | **Story Points**: 5 | **Dépendances**: Aucune

**Description**:
Créer un composant card compact et unifié pour tous les types de devices sur le plan.

**Spécifications visuelles**:

- **Température**: Icône thermomètre + `23.5°C` + `45%` humidité
- **Présence**: Cercle coloré (jaune=présent, gris=absent) + texte court
- **Switch**: Icône ampoule ou engrenage + fond coloré si ON
- **Energy meter**: Icône éclair + puissance actuelle en W
- **Coordinateur**: Icône antenne

**Bordures**:

- Router: Bordure double épaisseur bleue
- Coordinateur: Bordure double épaisseur orange/jaune
- Autres: Bordure standard grise

**Features**:

- Badge LQI mini en haut à droite (réutilise `Badge.svelte` variant `mini`)
- Toggle Turbo si device supporte (comme `DeviceNode.svelte` actuel)
- Onclick → navigation vers page détail device

**Critères d'acceptation**:

- [x] Support des 5 types de devices
- [x] Responsive et compact (~120-150px largeur)
- [x] Animations hover subtiles
- [x] Props typées TypeScript

**Fichiers créés**:

- `frontend/src/lib/floor-map/FloorMapCard.svelte`

**Status**: ✅ COMPLETED

---

#### Ticket #FM-006 : Composant SVGBackgroundNode

**Priorité**: P0 | **Story Points**: 3 | **Dépendances**: FM-004

**Description**:
Créer un custom node SvelteFlow pour afficher le SVG du plan en arrière-plan.

**Spécifications**:

- Node type custom `background-svg`
- Position fixe (0,0), non-draggable, non-selectable
- Z-index le plus bas
- Responsive au zoom SvelteFlow

**Critères d'acceptation**:

- [x] SVG rendu correctement dans le canvas
- [x] Ne bloque pas les interactions avec les device cards
- [x] Support du pan/zoom
- [x] Fallback si pas de plan uploadé

**Fichiers créés**:

- `frontend/src/lib/floor-map/SVGBackgroundNode.svelte`
- `frontend/src/lib/api/floor-map.ts` (API helper functions)

**Status**: ✅ COMPLETED

---

#### Ticket #FM-007 : Store positions avec sync backend

**Priorité**: P0 | **Story Points**: 3 | **Dépendances**: FM-003

**Description**:
Migrer le store `networkTopology.ts` de localStorage vers sync avec le backend.

**Comportement**:

- Chargement initial depuis API
- Update optimiste local + persist async vers backend
- Debounce des saves (300ms) pour éviter spam API pendant drag
- Fallback localStorage si API indisponible

**Critères d'acceptation**:

- [x] Sync bidirectionnelle frontend ↔ backend
- [x] Pas de perte de positions au refresh
- [x] Performance smooth pendant drag (optimistic updates)

**Fichiers à modifier**:

- `frontend/src/lib/stores/networkTopology.ts` → `floorMapStore.ts`
- `frontend/src/lib/api/floor-map.ts` - Nouveau fichier

**Status**: ✅ COMPLETED

---

#### Ticket #FM-008 : Composant FloorMapView principal

**Priorité**: P0 | **Story Points**: 5 | **Dépendances**: FM-005, FM-006, FM-007

**Description**:
Refactorer `NetworkGraph.svelte` en `FloorMapView.svelte` intégrant tous les nouveaux composants.

**Features**:

- Toggle affichage des connexions réseau (switch en haut à droite)
- Upload SVG via drag & drop ou file picker
- Tous les devices positionnables
- Persistence des positions

**Critères d'acceptation**:

- [x] Rendu correct du plan SVG en fond
- [x] Tous les devices affichés avec FloorMapCard
- [x] Drag & drop pour repositionner
- [x] Toggle connexions réseau fonctionnel
- [x] Upload SVG fonctionnel

**Fichiers à créer/modifier**:

- `frontend/src/lib/floor-map/FloorMapView.svelte`
- Migration depuis `NetworkGraph.svelte`

**Status**: ✅ COMPLETED

---

### Phase 3 : Pages Détail Device (P1)

#### Ticket #FM-009 : Page détail capteur température

**Priorité**: P1 | **Story Points**: 5 | **Dépendances**: FM-008

**Description**:
Créer la page détail accessible au clic sur une card capteur température.

**Contenu**:

- Header: Nom device (éditable), icône
- Métriques actuelles: Température, Humidité
- État: Battery, LQI, last_received
- Graphique historique avec sélecteur time range (réutilise `UnifiedChart.svelte`)

**Critères d'acceptation**:

- [x] Affichage de toutes les métriques
- [x] Graphique interactif avec zoom
- [x] Time range selectionnable (24h, 1w, 1m, 1y)
- [x] Navigation retour vers floor map

**Fichiers créés/modifiés**:

- `frontend/src/lib/device-detail/SensorDetailPage.svelte` - New component
- `frontend/src/App.svelte` - Route `/sensor/:id` added
- `frontend/src/lib/floor-map/FloorMapView.svelte` - Navigation to detail page

**Status**: ✅ COMPLETED

---

#### Ticket #FM-010 : Page détail switch

**Priorité**: P1 | **Story Points**: 5 | **Dépendances**: FM-008

**Description**:
Créer la page détail accessible au clic sur une card switch.

**Contenu**:

- Header: Nom device (éditable), icône
- État actuel: ON/OFF avec bouton toggle
- Métriques: Battery (si présente), LQI, last_received
- Tableau des règles d'automation liées à ce switch
- Bouton "Ajouter une règle" (réutilise `RuleEditor.svelte`)

**Critères d'acceptation**:

- [x] Toggle switch fonctionnel
- [x] Liste des règles d'automation
- [x] Création de règle depuis cette page
- [x] Navigation retour vers floor map

**Fichiers à créer**:

- `frontend/src/lib/device-detail/SwitchDetailPage.svelte`

**Status**: ✅ COMPLETED

---

#### Ticket #FM-011 : Page détail energy meter

**Priorité**: P1 | **Story Points**: 3 | **Dépendances**: FM-008

**Description**:
Créer la page détail accessible au clic sur une card energy meter.

**Contenu**:

- Header: Nom device (éditable), icône
- Métriques actuelles: Power, Voltage, Current, Energy totale
- Graphique historique puissance

**Critères d'acceptation**:

- [ ] Toutes les métriques energy affichées
- [ ] Graphique puissance dans le temps
- [ ] Time range selectionnable

**Fichiers à créer**:

- `frontend/src/lib/device-detail/EnergyDetailPage.svelte`

---

#### Ticket #FM-012 : Page détail capteur présence

**Priorité**: P1 | **Story Points**: 3 | **Dépendances**: FM-008

**Description**:
Créer la page détail accessible au clic sur une card capteur présence.

**Contenu**:

- Header: Nom device (éditable), icône
- État actuel: Présence détectée oui/non
- Métriques: Illumination (si disponible), Battery, LQI, last_received
- Historique des détections

**Critères d'acceptation**:

- [ ] État présence affiché clairement
- [ ] Historique des événements présence
- [ ] Timeline visuelle

**Fichiers à créer**:

- `frontend/src/lib/device-detail/PresenceDetailPage.svelte`

---

### Phase 4 : Polish & UX (P2)

#### Ticket #FM-013 : Animations et transitions

**Priorité**: P2 | **Story Points**: 2 | **Dépendances**: FM-008

**Description**:
Ajouter des animations fluides pour améliorer l'UX.

**Animations**:

- Transition fade-in des cards au chargement
- Animation état switch (toggle smooth)
- Highlight card pendant drag
- Transition couleur présence (jaune ↔ gris)

**Fichiers à modifier**:

- `FloorMapCard.svelte` - CSS animations

---

#### Ticket #FM-014 : Gestion erreurs et états vides

**Priorité**: P2 | **Story Points**: 2 | **Dépendances**: FM-008

**Description**:
Gérer proprement les cas limites et erreurs.

**Cas à gérer**:

- Pas de plan SVG uploadé → placeholder avec CTA upload
- Device sans position → placement automatique
- Erreur sync positions → notification toast
- Device offline → style visuel distinct

**Fichiers à modifier**:

- `FloorMapView.svelte`
- `FloorMapCard.svelte`

---

#### Ticket #FM-015 : Responsive mobile

**Priorité**: P2 | **Story Points**: 3 | **Dépendances**: FM-008

**Description**:
Adapter la vue floor map pour mobile/tablette.

**Adaptations**:

- Cards plus compactes sur petit écran
- Touch-friendly drag & drop
- Pinch-to-zoom sur le plan
- Bottom sheet pour détail device (au lieu de page)

---

### Phase 5 : Documentation (P3)

#### Ticket #FM-016 : Documentation utilisateur

**Priorité**: P3 | **Story Points**: 1 | **Dépendances**: FM-014

**Description**:
Documenter l'utilisation de la feature floor map.

**Contenu**:

- Comment uploader un plan SVG
- Comment positionner les devices
- Explication des indicateurs visuels

---

## Ordre d'implémentation recommandé

```
Week 1: Infrastructure
├── FM-001 (Schema positions)        ─┬─► FM-003 (API positions)
├── FM-002 (Schema floor plan)       ─┴─► FM-004 (API floor plan)
└── FM-005 (FloorMapCard)            ────► En parallèle

Week 2: Intégration
├── FM-006 (SVGBackgroundNode)
├── FM-007 (Store sync)
└── FM-008 (FloorMapView)            ────► Intégration finale

Week 3: Pages détail
├── FM-009 (Détail température)
├── FM-010 (Détail switch)
├── FM-011 (Détail energy)
└── FM-012 (Détail présence)

Week 4: Polish
├── FM-013 (Animations)
├── FM-014 (Erreurs/vides)
├── FM-015 (Responsive)
└── FM-016 (Documentation)
```

---

## Risques et mitigations

| Risque                            | Impact | Mitigation                             |
| --------------------------------- | ------ | -------------------------------------- |
| Performance SVG large             | Medium | Limiter taille upload, optimiser rendu |
| Sync positions conflits           | Low    | Dernier écrit gagne, pas de multi-user |
| SvelteFlow custom nodes complexes | Medium | POC technique avant dev complet        |

---

## Métriques de succès

- [ ] Temps de rendu floor map < 100ms
- [ ] Drag & drop smooth (60fps)
- [ ] Sync positions < 500ms
- [ ] 0 régression sur Network view existante

---

## Notes techniques

### Migration NetworkGraph → FloorMapView

L'approche recommandée est de **créer les nouveaux composants à côté** puis de basculer, plutôt que de refactorer in-place :

1. Créer `frontend/src/lib/floor-map/` avec tous les nouveaux composants
2. Ajouter une route `/floor-map` pour tester en parallèle
3. Une fois validé, remplacer l'import dans la page Network
4. Supprimer les anciens fichiers

### Réutilisation du Design System

- `Badge.svelte` variant `mini` → LQI indicator
- `Button.svelte` → Actions dans pages détail
- `Card.svelte` → Wrapper pour FloorMapCard
- Variables CSS existantes → Cohérence visuelle
