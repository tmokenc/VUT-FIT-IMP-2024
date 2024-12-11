use heapless::Vec;
use rand::prelude::*;

// Shape of a tetromino, it always has 4 blocks with coordination with the default offset
pub type TetrominoBlocks = [Coordination; 4];

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Coordination {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy)]
pub enum Tetromino {
    L,
    J,
    T,
    O,
    Z,
    S,
    I,
}

#[derive(Default, Clone, Copy)]
pub enum Rotation {
    #[default]
    Default,
    Left,
    Flipped,
    Right,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    Occured,
    #[default]
    Empty,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    Rotate,
}

#[derive(Default, PartialEq)]
pub enum BoardUpdate<const N: usize> {
    Full,
    Partial(Vec<(Coordination, Cell), N>),
    #[default]
    None,
}

pub enum State {
    New,
    Playing {
        piece: Tetromino,
        rotation: Rotation,
        offset: Coordination,
        queue: TetrominoQueue,
        score: u64,
    },
    GameOver {
        score: u64,
    },
}

pub struct Board<const C: usize, const R: usize> {
    inner: [[Cell; C]; R],
}

impl<const C: usize, const R: usize> Board<C, R> {
    const fn new() -> Self {
        Self {
            inner: [[Cell::Empty; C]; R],
        }
    }

    fn place(&mut self, blocks: TetrominoBlocks, offset: Coordination) -> u8 {
        for block in blocks {
            let x = block.x + offset.x;
            let y = block.y + offset.y;

            if y < 0 {
                continue;
            }

            self.inner[y as usize][x as usize] = Cell::Occured;
        }

        self.clear_full_lines()
    }

    fn clear_full_lines(&mut self) -> u8 {
        let mut new_board: [[Cell; C]; R] = [[Cell::Empty; C]; R];
        let mut new_board_line_index = R - 1;
        let mut removed_count = 0;

        // Copy the lines from current board to new Board, ignoring fully filled lines.
        for line_index in (0..R).rev() {
            if self.inner[line_index].iter().all(|&v| v == Cell::Occured) {
                removed_count += 1;
                continue;
            }

            new_board[new_board_line_index] = self.inner[line_index];
            new_board_line_index -= 1;
        }

        self.inner = new_board;
        removed_count
    }

    fn wall_bounce_offset_modifier(&self, blocks: TetrominoBlocks, offset: Coordination) -> i16 {
        let mut modifier = 0;

        for block in blocks {
            let x = block.x + offset.x;

            if x < 0 {
                modifier = modifier.max(-x);
            } else if x >= C as i16 {
                modifier = modifier.min(C as i16 - x - 1);
            }
        }

        modifier
    }

    fn can_move_in(&self, blocks: TetrominoBlocks, offset: Coordination) -> bool {
        for block in blocks {
            let x = block.x + offset.x;
            let y = block.y + offset.y;

            // Ignore hidden pieces on the top
            if y < 0 {
                continue;
            }

            if y >= R as i16 || x < 0 || x >= C as i16 {
                return false;
            }

            if self.inner[y as usize][x as usize] == Cell::Occured {
                return false;
            }
        }

        true
    }

    pub fn iter(&self) -> BoardIter<'_, C, R> {
        BoardIter {
            board: self,
            current_coor: Coordination { x: 0, y: 0 },
        }
    }
}

pub struct BoardIter<'a, const C: usize, const R: usize> {
    board: &'a Board<C, R>,
    current_coor: Coordination,
}

impl<'a, const COL: usize, const ROW: usize> Iterator for BoardIter<'a, COL, ROW> {
    type Item = Coordination;

    fn next(&mut self) -> Option<Self::Item> {
        let mut coor = self.current_coor;

        while (coor.x as usize) < COL && (coor.y as usize) < ROW {
            self.current_coor.x += 1;

            if self.current_coor.x as usize >= COL {
                self.current_coor.x = 0;
                self.current_coor.y += 1;
            }

            if self.board.inner[coor.y as usize][coor.x as usize] == Cell::Occured {
                return Some(coor);
            }

            coor = self.current_coor;
        }

        None
    }
}

pub struct TetrominoQueue {
    queue: Vec<Tetromino, 7>,
}

impl TetrominoQueue {
    fn new() -> Self {
        Self { queue: Vec::new() }
    }

    fn init(&mut self, rng: &mut impl Rng) {
        let _ = self.queue.extend_from_slice(&[
            Tetromino::J,
            Tetromino::L,
            Tetromino::S,
            Tetromino::Z,
            Tetromino::T,
            Tetromino::O,
            Tetromino::I,
        ]);

        self.queue.shuffle(rng);
    }

    fn next(&mut self, rng: &mut impl Rng) -> Tetromino {
        let result = self.queue.pop().unwrap();

        if self.queue.is_empty() {
            self.init(rng);
        }

        result
    }

    pub fn peek(&self) -> Tetromino {
        *self.queue.last().unwrap()
    }
}

pub struct Tetris<const C: usize, const R: usize, Rng: RngCore> {
    pub board: Board<C, R>,
    pub state: State,
    rng: Option<Rng>,
}

impl<const C: usize, const R: usize, Rng: RngCore> Tetris<C, R, Rng> {
    pub const fn new() -> Self {
        Self {
            board: Board::new(),
            state: State::New,
            rng: None,
        }
    }

    pub fn set_rng(&mut self, rng: Rng) {
        self.rng = Some(rng);
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, State::Playing { .. })
    }

    pub fn start(&mut self) {
        if self.is_playing() || self.rng.is_none() {
            return;
        }

        let mut queue = TetrominoQueue::new();
        self.board = Board::new();
        queue.init(self.rng.as_mut().unwrap());

        self.state = State::Playing {
            piece: Tetromino::J,
            rotation: Rotation::Default,
            score: 0,
            offset: Coordination { x: 5, y: 0 },
            queue,
        };

        self.spawn_new_piece();
    }

    /// Drop speed in milliseconds
    /// Hard code 3 seconds for now
    #[inline]
    pub fn drop_speed(&self) -> u64 {
        1000
    }

    pub fn get_current_tetromino_position(&self) -> TetrominoBlocks {
        if let State::Playing {
            piece,
            rotation,
            offset,
            ..
        } = self.state
        {
            get_tetromino_blocks(piece, rotation).map(|block| Coordination {
                x: block.x + offset.x,
                y: block.y + offset.y,
            })
        } else {
            [Coordination::default(); 4]
        }
    }

    fn spawn_new_piece(&mut self) {
        let mut is_gameover: Option<State> = None;

        if let State::Playing {
            ref mut piece,
            ref mut rotation,
            ref mut offset,
            ref mut queue,
            score,
            ..
        } = self.state
        {
            *rotation = Rotation::Default;
            *offset = Coordination {
                x: (C / 2) as i16,
                y: 0,
            };

            *piece = queue.next(self.rng.as_mut().unwrap());

            if !self
                .board
                .can_move_in(get_tetromino_blocks(*piece, *rotation), *offset)
            {
                is_gameover = Some(State::GameOver { score });
            }
        }

        if let Some(is_gameover) = is_gameover {
            self.state = is_gameover;
        }
    }

    pub fn act(&mut self, action: Action) -> BoardUpdate<16> {
        let previous_blocks = self.get_current_tetromino_position();

        let State::Playing {
            ref mut piece,
            ref mut rotation,
            ref mut offset,
            ref mut score,
            ..
        } = self.state
        else {
            return BoardUpdate::None;
        };

        let mut board_update = BoardUpdate::None;
        let mut updated = false;

        match action {
            Action::MoveLeft => {
                let blocks = get_tetromino_blocks(*piece, *rotation);
                let mut new_offset = *offset;
                new_offset.x -= 1;

                if self.board.can_move_in(blocks, new_offset) {
                    offset.x -= 1;
                    board_update = BoardUpdate::get_partial_update(
                        previous_blocks,
                        self.get_current_tetromino_position(),
                    );
                }
            }

            Action::MoveRight => {
                let blocks = get_tetromino_blocks(*piece, *rotation);
                let mut new_offset = *offset;
                new_offset.x += 1;

                if self.board.can_move_in(blocks, new_offset) {
                    offset.x += 1;
                    updated = true;
                }
            }

            Action::SoftDrop => {
                let blocks = get_tetromino_blocks(*piece, *rotation);
                let mut new_offset = *offset;
                new_offset.y += 1;

                if self.board.can_move_in(blocks, new_offset) {
                    offset.y += 1;
                    updated = true;
                } else {
                    let cleared_lines = self.board.place(blocks, *offset);
                    if cleared_lines > 0 {
                        *score += cleared_lines as u64;
                    }

                    self.spawn_new_piece();
                    return BoardUpdate::Full;
                }
            }

            Action::HardDrop => {
                // increase y offset until it cannot be moved in
                let blocks = get_tetromino_blocks(*piece, *rotation);
                let mut new_offset = *offset;
                new_offset.y += 1;

                while self.board.can_move_in(blocks, new_offset) {
                    new_offset.y += 1;
                }

                *offset = new_offset;
                offset.y -= 1; // undo the last increment

                // let the SoftDrop handle the rest
                return self.act(Action::SoftDrop);
            }
            Action::Rotate => {
                let new_rotation = match rotation {
                    Rotation::Default => Rotation::Left,
                    Rotation::Left => Rotation::Flipped,
                    Rotation::Flipped => Rotation::Right,
                    Rotation::Right => Rotation::Default,
                };

                let blocks = get_tetromino_blocks(*piece, new_rotation);

                let mut new_offset = *offset;
                new_offset.x += self.board.wall_bounce_offset_modifier(blocks, *offset);

                if self.board.can_move_in(blocks, new_offset) {
                    *rotation = new_rotation;
                    *offset = new_offset;
                    updated = true;
                }
            }
        }

        if updated && board_update == BoardUpdate::None {
            board_update.merge(BoardUpdate::get_partial_update(
                previous_blocks,
                self.get_current_tetromino_position(),
            ));
        }

        board_update
    }
}

pub fn get_tetromino_blocks(piece: Tetromino, rotation: Rotation) -> TetrominoBlocks {
    let data = match (piece, rotation) {
        (Tetromino::O, _) => [(0, 0), (1, 0), (0, 1), (1, 1)],

        (Tetromino::I, Rotation::Left | Rotation::Right) => [(0, 1), (1, 1), (2, 1), (3, 1)],
        (Tetromino::I, _) => [(1, 0), (1, 1), (1, 2), (1, 3)],

        (Tetromino::S, Rotation::Default) => [(0, 0), (1, 0), (1, 1), (2, 1)],
        (Tetromino::S, Rotation::Left) => [(2, 0), (2, 1), (1, 1), (1, 2)],
        (Tetromino::S, Rotation::Flipped) => [(2, 2), (1, 2), (1, 1), (0, 1)],
        (Tetromino::S, Rotation::Right) => [(0, 2), (0, 1), (1, 1), (1, 0)],

        (Tetromino::Z, Rotation::Default) => [(1, 0), (2, 0), (0, 1), (1, 1)],
        (Tetromino::Z, Rotation::Left) => [(2, 1), (2, 2), (1, 0), (1, 1)],
        (Tetromino::Z, Rotation::Flipped) => [(1, 2), (0, 2), (2, 1), (1, 1)],
        (Tetromino::Z, Rotation::Right) => [(0, 1), (0, 0), (1, 2), (1, 1)],

        (Tetromino::L, Rotation::Default) => [(0, 2), (1, 2), (1, 1), (1, 0)],
        (Tetromino::L, Rotation::Left) => [(0, 0), (0, 1), (1, 1), (2, 1)],
        (Tetromino::L, Rotation::Flipped) => [(2, 0), (1, 0), (1, 1), (1, 2)],
        (Tetromino::L, Rotation::Right) => [(2, 2), (2, 1), (1, 1), (0, 1)],

        (Tetromino::T, Rotation::Default) => [(1, 0), (0, 1), (1, 1), (2, 1)],
        (Tetromino::T, Rotation::Left) => [(2, 1), (1, 0), (1, 1), (1, 2)],
        (Tetromino::T, Rotation::Flipped) => [(1, 2), (2, 1), (1, 1), (0, 1)],
        (Tetromino::T, Rotation::Right) => [(0, 1), (1, 2), (1, 1), (1, 0)],

        (Tetromino::J, Rotation::Default) => [(0, 0), (1, 2), (1, 1), (1, 0)],
        (Tetromino::J, Rotation::Left) => [(2, 0), (0, 1), (1, 1), (2, 1)],
        (Tetromino::J, Rotation::Flipped) => [(2, 2), (1, 0), (1, 1), (1, 2)],
        (Tetromino::J, Rotation::Right) => [(0, 2), (2, 1), (1, 1), (0, 1)],
    };

    data.map(|v| Coordination { x: v.0, y: v.1 })
}

impl<const N: usize> BoardUpdate<N> {
    fn get_partial_update(
        previous_blocks: TetrominoBlocks,
        current_blocks: TetrominoBlocks,
    ) -> Self {
        let mut list = Vec::new();

        for block in previous_blocks {
            if !current_blocks.contains(&block) {
                list.push((block, Cell::Empty)).unwrap();
            }
        }

        for block in current_blocks {
            if !previous_blocks.contains(&block) {
                list.push((block, Cell::Occured)).unwrap();
            }
        }

        BoardUpdate::Partial(list)
    }

    pub fn merge(&mut self, other: Self) {
        let mut require_full_update = false;

        match self {
            BoardUpdate::None => *self = other,
            BoardUpdate::Full => (),
            BoardUpdate::Partial(ref mut self_data) => match other {
                BoardUpdate::None => (),
                BoardUpdate::Full => require_full_update = true,
                BoardUpdate::Partial(other_data) => {
                    'outer: for block in other_data {
                        for current_block in self_data.iter_mut() {
                            if current_block.0 == block.0 {
                                current_block.1 = block.1;
                                continue 'outer;
                            }
                        }

                        // Require full update if the vector is completely full
                        if self_data.push(block).is_err() {
                            require_full_update = true;
                            break;
                        }
                    }
                }
            },
        }

        if require_full_update {
            *self = BoardUpdate::Full;
        }
    }
}
