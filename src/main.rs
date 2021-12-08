use std::io::{stdout, Write};
use rand::Rng;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{read, Event, KeyCode, EnableMouseCapture, DisableMouseCapture, MouseEventKind, MouseButton};
use crossterm::cursor::{MoveTo, Show, Hide};
use crossterm::style::{SetForegroundColor, Color, SetBackgroundColor};
use crossterm::{Result, execute, queue};

const w: usize = 30;
const h: usize = 16;

fn main() -> Result<()>
{
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;

    let res = Minesweeper::new().run_game();

    if let Ok(false) = res
    {
        println!("YOU LOSE !");
        read()?; // Wait for the user to press a key
    }

    execute!(stdout, Show, DisableMouseCapture, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    res.map(|_|())
}

pub enum InputAction
{
    Move(usize, usize),
    Flag,
    Reveal,
    RevealArea,
    Quit
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum CellState
{
    Uncovered,
    Covered,
    Flagged
}

impl CellState
{
    pub fn is_covered(self) -> bool
    {
        self != Self::Uncovered
    }

    pub fn is_flagged(self) -> bool
    {
        self == Self::Flagged
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum CellContent
{
    Mine,
    Number(u8)
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Cell
{
    content: CellContent,
    state: CellState
}

impl Cell
{
    pub const EMPTY: Self = Cell { content: CellContent::Number(0), state: CellState::Covered };
    
    fn print_cell(self, output: &mut impl Write) -> Result<()>
    {
        match self.state
        {
            CellState::Flagged =>
            {
                queue!(output, SetForegroundColor(Color::Red))?;
                write!(output, "P")?;
            },
            CellState::Covered =>
            {
                queue!(output, SetForegroundColor(Color::Reset))?;
                write!(output, "■")?;
            },
            CellState::Uncovered =>
            {
                match self.content
                {
                    CellContent::Mine =>
                    {
                        queue!(output, SetForegroundColor(Color::Red))?;
                        write!(output, "*")?;
                    },
                    CellContent::Number(0) =>
                    {
                        write!(output, " ")?;
                    },
                    CellContent::Number(nb) =>
                    {
                        queue!(output, SetForegroundColor(Color::Cyan))?;
                        write!(output, "{}", nb)?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct Minesweeper
{
    grid: [[Cell; h]; w],
    generated: bool,
    mine_count: u32,
    flag_count: u32,
    px: usize,
    py: usize
}

impl Minesweeper
{
    pub fn new() -> Self
    {
        Self
        {
            grid: [[Cell::EMPTY; h]; w],
            generated: false,
            mine_count: 99,
            flag_count: 0,
            px: w / 2,
            py: h / 2
        }
    }

    fn print_grid(&self, output: &mut impl Write) -> Result<()>
    {
        for y in 0..h
        {
            queue!(output, MoveTo(0, y as u16))?;
            for x in 0..w
            {
                if x == self.px && y == self.py
                {
                    queue!(output, SetBackgroundColor(Color::DarkGrey))?;
                }

                self.grid[x][y].print_cell(output)?;
                
                queue!(output, SetBackgroundColor(Color::Reset))?;
                write!(output, " ")?;
            }
        }
        queue!(output, MoveTo(0, h as u16))?;
        write!(output, "{}   ", self.mine_count as i32 - self.flag_count as i32)?;
        output.flush()?;
        Ok(())
    }

    fn count_neighbors(&self, x: usize, y: usize, pred: impl Fn(Cell) -> bool) -> u8
    {
        let mut n = 0;
        for nx in x.checked_sub(1).unwrap_or(0)..=x+1
        {
            for ny in y.checked_sub(1).unwrap_or(0)..=y+1
            {
                if nx < w && ny < h && pred(self.grid[nx][ny])
                {
                    n += 1;
                }
            }
        }
        n
    }

    fn generate_grid(&mut self)
    {
        let mut rng = rand::thread_rng();
        for _mi in 0..self.mine_count
        {
            let mut x;
            let mut y;
            loop
            {
                x = rng.gen_range(0..w);
                y = rng.gen_range(0..h);
                if self.grid[x][y].content != CellContent::Mine &&
                ((x as isize - self.px as isize).abs() >  1 || (y as isize - self.py as isize).abs() >  1)
                {
                    break;
                }
            }

            self.grid[x][y].content = CellContent::Mine;
        }
        
        for x in 0..w
        {
            for y in 0..h
            {
                if self.grid[x][y].content != CellContent::Mine
                {
                    let n = self.count_neighbors(x, y, |c| c.content == CellContent::Mine);
                    self.grid[x][y].content = CellContent::Number(n);
                }
            }
        }
        self.generated = true;
    }

    fn mark_cell(&mut self, x: usize, y: usize)
    {
        match self.grid[x][y].state
        {
            CellState::Covered =>
            {
                self.grid[x][y].state = CellState::Flagged;
                self.flag_count += 1;
            },
            CellState::Flagged =>
            {
                self.grid[x][y].state = CellState::Covered;
                self.flag_count -= 1;
            },
            CellState::Uncovered => { }
        }
    }

    fn reveal_area(&mut self, x: usize, y: usize) -> Result<bool>
    {
        if let (CellState::Uncovered, CellContent::Number(nb)) = (self.grid[x][y].state, self.grid[x][y].content)
        {
            let flags = self.count_neighbors(x, y, |c| c.state.is_flagged());
            if nb != flags
            {
                return Ok(true)
            }
            
            let mut res = true;
            for nx in x.checked_sub(1).unwrap_or(0)..=x+1
            {
                for ny in y.checked_sub(1).unwrap_or(0)..=y+1
                {
                    if nx < w && ny < h && !self.grid[nx][ny].state.is_flagged()
                    {
                        res &= self.reveal(nx, ny)?
                    }
                }
            }
            return Ok(res)
        }
        return Ok(true)
    }

    fn reveal(&mut self, x: usize, y: usize) -> Result<bool>
    {
        if !self.generated
        {
            self.generate_grid();
        }

        if self.grid[x][y].state != CellState::Covered
        {
            return Ok(true);
        }

        self.grid[x][y].state = CellState::Uncovered;
        match self.grid[x][y].content
        {
            CellContent::Mine =>
            {
                Ok(false)
            },
            CellContent::Number(nb) =>
            {
                if nb == 0
                {
                    // Propagate reveal
                    for nx in x.checked_sub(1).unwrap_or(0)..=x+1
                    {
                        for ny in y.checked_sub(1).unwrap_or(0)..=y+1
                        {
                            if nx < w && ny < h
                            {
                                self.reveal(nx, ny)?;
                            }
                        }
                    }
                }
                Ok(true)
            }
        }
    }

    fn read_input(&self, event: Event) -> Vec<InputAction>
    {
        match event
        {
            Event::Key(ke) =>
            {
                match ke.code
                {
                    KeyCode::Char('q') => vec![InputAction::Quit],
                    KeyCode::Up => vec![InputAction::Move(self.px, self.py.checked_sub(1).unwrap_or(0))],
                    KeyCode::Down =>
                    {
                        let mut py = self.py + 1;
                        if py >= h
                        {
                            py = h - 1;
                        }
                        vec![InputAction::Move(self.px, py)]
                    },
                    KeyCode::Right =>
                    {
                        let mut px = self.px + 1;
                        if px >= w
                        {
                            px = w - 1;
                        }
                        vec![InputAction::Move(px, self.py)]
                    },
                    KeyCode::Left => vec![InputAction::Move(self.px.checked_sub(1).unwrap_or(0), self.py)],
                    KeyCode::Char(' ' | 's') => vec![InputAction::Reveal],
                    KeyCode::Char('!' | 'z') => vec![InputAction::Flag],
                    KeyCode::Char('d') => vec![InputAction::RevealArea],
                    _ => vec![]
                }
            },
            Event::Mouse(me) =>
            {
                let npx = (me.column / 2) as usize;
                let npy = me.row as usize;
                if npx >= w || npy >= h || me.column % 2 != 0
                {
                    return vec![];
                }
                let mut actions = vec![InputAction::Move(npx, npy)];
                match me.kind
                {
                    MouseEventKind::Down(MouseButton::Left) => actions.push(InputAction::Reveal),
                    MouseEventKind::Down(MouseButton::Right) => actions.push(InputAction::Flag),
                    MouseEventKind::Down(MouseButton::Middle) => actions.push(InputAction::RevealArea),
                    _ => { }
                }
                actions
            }
            _ => vec![]
        }
    }

    pub fn run_game(mut self) -> Result<bool>
    {
        let mut stdout = stdout();
        loop
        {
            self.print_grid(&mut stdout)?;
            
            let actions = self.read_input(read()?);
            for action in actions
            {
                match action
                {
                    InputAction::Move(x, y) =>
                    {
                        self.px = x;
                        self.py = y;
                    },
                    InputAction::Reveal =>
                    {
                        if !self.reveal(self.px, self.py)?
                        {
                            return Ok(false);
                        }
                    },
                    InputAction::RevealArea =>
                    {
                        if !self.reveal_area(self.px, self.py)?
                        {
                            return Ok(false);
                        }
                    },
                    InputAction::Flag =>
                    {
                        self.mark_cell(self.px, self.py);
                    },
                    InputAction::Quit =>
                    {
                        return Ok(true);
                    }
                }
            }
        }
    }
}