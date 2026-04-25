use bevy::prelude::*;
use std::cmp::PartialEq;
use std::collections::HashMap;

type Cell = (isize, isize);

enum Direction {
    Cardinal,
    Intercardinal,
}

impl Direction {
    fn valid_positions(&self, (x1, y1): Cell, (x2, y2): Cell) -> bool {
        match self {
            Direction::Cardinal => (x1 - x2).abs() == 0 || (y1 - y2).abs() == 0,
            Direction::Intercardinal => true,
        }
    }
}

#[derive(PartialEq)]
enum GapMode {
    Include,
    Exclude,
}

#[derive(Component)]
#[require(Transform)]
struct Grid2d {
    width: Option<usize>,
    height: Option<usize>,
    size: Vec2,
    gap: Vec2,
    gap_mode: GapMode,
    entities: HashMap<Cell, Entity>,
}

#[derive(EntityEvent)]
struct Grid2dHover {
    #[event_target]
    grid: Entity,
    cell: Cell,
    entity: Option<Entity>,
}

impl Grid2d {
    pub fn new(width: Option<usize>, height: Option<usize>, size: Vec2, gap: Vec2) -> Self {
        Self {
            width,
            height,
            size,
            gap,
            gap_mode: GapMode::Exclude,
            entities: HashMap::new(),
        }
    }

    pub fn get_points_for_cell(&self, cell: Cell) -> Vec<Vec2> {
        let cell_width = self.size.x + self.gap.x;
        let cell_height = self.size.y + self.gap.y;
        vec![
            Vec2::new(cell.0 as f32 * cell_width, cell.1 as f32 * cell_height),
            Vec2::new(
                (cell.0 + 1) as f32 * cell_width - self.gap.x,
                cell.1 as f32 * cell_height,
            ),
            Vec2::new(
                (cell.0 + 1) as f32 * cell_width - self.gap.x,
                (cell.1 + 1) as f32 * cell_height - self.gap.y,
            ),
            Vec2::new(
                cell.0 as f32 * cell_width,
                (cell.1 + 1) as f32 * cell_height - self.gap.y,
            ),
        ]
    }

    pub fn get_cell_for_offset(&self, offset: Vec2) -> Option<Cell> {
        let cell_width = self.size.x + self.gap.x;
        let cell_height = self.size.y + self.gap.y;
        if let Some(x) = self.width
            && (offset.x < 0.0 || offset.x >= x as f32 * cell_width - self.gap.x)
        {
            return None;
        }
        if let Some(y) = self.height
            && (offset.y < 0.0 || offset.y >= y as f32 * cell_height - self.gap.y)
        {
            return None;
        }
        let cell = (
            (offset.x / cell_width).floor() as isize,
            (offset.y / cell_height).floor() as isize,
        );
        let cell_start_x = cell.0 as f32 * cell_width;
        let cell_end_x = cell_start_x + self.size.x;
        let cell_start_y = cell.1 as f32 * cell_height;
        let cell_end_y = cell_start_y + self.size.y;

        match self.gap_mode {
            GapMode::Include => {
                let cell_with_gap_end_x = cell_end_x + self.gap.x / 2.0;
                let cell_with_gap_end_y = cell_end_y + self.gap.y / 2.0;
                Some((
                    cell.0 + (cell_with_gap_end_x < offset.x) as isize,
                    cell.1 + (cell_with_gap_end_y < offset.y) as isize,
                ))
            }
            GapMode::Exclude => {
                let distance_x = offset.x - cell_start_x;
                let distance_y = offset.y - cell_start_y;
                if distance_x > self.size.x || distance_y > self.size.y {
                    None
                } else {
                    Some(cell)
                }
            }
        }
    }

    pub fn orthogonal_neighbors(&self, position: Cell) -> Vec<(Cell, Entity)> {
        self.neighbors(position, Direction::Cardinal)
    }

    pub fn all_neighbors(&self, position: Cell) -> Vec<(Cell, Entity)> {
        self.neighbors(position, Direction::Intercardinal)
    }

    pub fn neighbors(&self, position: Cell, direction: Direction) -> Vec<(Cell, Entity)> {
        self.range(position, direction, 1)
            .into_iter()
            .filter(|(pos, _)| pos != &position)
            .collect()
    }

    // TODO: use iterator instead of vec?
    pub fn range(
        &self,
        (x, y): Cell,
        direction: Direction,
        distance: usize,
    ) -> Vec<(Cell, Entity)> {
        let mut elements = vec![];
        for pos_x in x - distance as isize..x + distance as isize {
            for pos_y in y - distance as isize..y + distance as isize {
                let position = (pos_x, pos_y);
                if direction.valid_positions((x, y), position)
                    && let Some(element) = self.entities.get(&position)
                {
                    elements.push((position, *element));
                }
            }
        }
        elements
    }
}

fn hover_over_grids(
    grids: Query<(Entity, &Grid2d, &GlobalTransform)>,
    mouse_position: Res<MouseWorldPosition>,
    mut commands: Commands,
) {
    for (entity, grid, transform) in grids.iter() {
        let offset = transform_to_local_2d(transform, mouse_position.0);
        let Some(cell) = grid.get_cell_for_offset(offset) else {
            continue;
        };
        commands.trigger(Grid2dHover {
            grid: entity,
            cell,
            entity: grid.entities.get(&cell).copied(),
        });
    }
}

fn draw_hovered_cell(
    hover: On<Grid2dHover>,
    grid: Query<(&Grid2d, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    let (grid, transform) = grid.get(hover.grid).expect("grid");
    let points = grid
        .get_points_for_cell(hover.cell)
        .iter()
        .map(|point| transform.transform_point(point.extend(0.0)).truncate())
        .collect::<Vec<_>>();
    gizmos.lineloop_2d(points, Color::WHITE);
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn(Grid2d::new(
            Some(3),
            Some(3),
            Vec2::new(50.0, 50.0),
            Vec2::splat(100.0),
        ))
        .observe(draw_hovered_cell);
}

#[derive(Resource)]
struct MouseWorldPosition(Vec2);

impl Default for MouseWorldPosition {
    fn default() -> Self {
        MouseWorldPosition(Vec2::default())
    }
}

fn update_mouse_position(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window>,
    mut mouse_position: ResMut<MouseWorldPosition>,
) {
    let (camera, camera_transform) = *camera_query;
    if let Some(cursor_position) = window.cursor_position()
        && let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    {
        mouse_position.0 = world_pos;
    }
}

fn transform_to_local_2d(transform: &GlobalTransform, point: Vec2) -> Vec2 {
    transform
        .to_matrix()
        .inverse()
        .transform_point(point.extend(0.0))
        .truncate()
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, update_mouse_position)
        .add_systems(Update, hover_over_grids)
        .insert_resource(MouseWorldPosition::default())
        .run();
}
