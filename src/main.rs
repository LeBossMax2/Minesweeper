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
}

pub struct Minesweeper
{
    grid: [[Cell; h]; w],
    generated: bool,
    px: usize,
    py: usize,
    mine_count: u32,
    flag_count: u32
}

impl Minesweeper
{
    pub fn new() -> Self
    {
        Self
        {
            mine_count: 99,
            grid: [[Cell::EMPTY; h]; w],

            generated: false,
            px: w / 2,
            py: h / 2,
            flag_count: 0
        }
    }

    fn print_grid(&self) -> Result<()>
    {
        let mut stdout = stdout();
        for y in 0..h
        {
            queue!(stdout, MoveTo(0, y as u16))?;
            for x in 0..w
            {
                if x == self.px && y == self.py
                {
                    queue!(stdout, SetBackgroundColor(Color::DarkGrey))?;
                }

                match self.grid[x][y].state
                {
                    CellState::Flagged =>
                    {
                        queue!(stdout, SetForegroundColor(Color::Red))?;
                        print!("P");
                    },
                    CellState::Covered =>
                    {
                        queue!(stdout, SetForegroundColor(Color::Reset))?;
                        print!("■");
                    },
                    CellState::Uncovered =>
                    {
                        match self.grid[x][y].content
                        {
                            CellContent::Mine =>
                            {
                                queue!(stdout, SetForegroundColor(Color::Red))?;
                                print!("*");
                            },
                            CellContent::Number(0) =>
                            {
                                print!(" ");
                            },
                            CellContent::Number(nb) =>
                            {
                                queue!(stdout, SetForegroundColor(Color::Cyan))?;
                                print!("{}", nb);
                            }
                        }
                    }
                }
                queue!(stdout, SetBackgroundColor(Color::Reset))?;
                print!(" ");
            }
        }
        queue!(stdout, MoveTo(0, h as u16))?;
        print!("{}   ", self.mine_count as i32 - self.flag_count as i32);
        stdout.flush()?;
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
        if let (CellState::Uncovered, CellContent::Number(nb)) = (self.grid[x][y].state, self.grid[x][y].content )
        {
            let flags = self.count_neighbors(x, y, |c| c.state.is_flagged());
            if nb == flags
            {
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
        }
        return Ok(true)
    }

    fn reveal(&mut self, x: usize, y: usize) -> Result<bool>
    {
        if !self.generated
        {
            self.generate_grid();
        }

        match self.grid[x][y].state
        {
            CellState::Uncovered | CellState::Flagged =>
            {
                Ok(true)
            },
            CellState::Covered =>
            {
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
        }
    }

    pub fn run_game(mut self) -> Result<bool>
    {
        loop
        {
            self.print_grid()?;
            
            match read()?
            {
                Event::Key(ke) =>
                {
                    match ke.code
                    {
                        KeyCode::Char('q') => return Ok(true),
                        KeyCode::Up => self.py = self.py.checked_sub(1).unwrap_or(0),
                        KeyCode::Down =>
                        {
                            self.py += 1;
                            if self.py >= h
                            {
                                self.py = h - 1;
                            }
                        },
                        KeyCode::Right =>
                        {
                            self.px += 1;
                            if self.px >= w
                            {
                                self.px = w - 1;
                            }
                        },
                        KeyCode::Left => self.px = self.px.checked_sub(1).unwrap_or(0),
                        KeyCode::Char(' ' | 's') =>
                        {
                            if !self.reveal(self.px, self.py)?
                            {
                                return Ok(false);
                            }
                        },
                        KeyCode::Char('!' | 'z') =>
                        {
                            self.mark_cell(self.px, self.py);
                        },
                        KeyCode::Char('d') =>
                        {
                            if !self.reveal_area(self.px, self.py)?
                            {
                                return Ok(false);
                            }
                        }
                        _ => { }
                    }
                },
                Event::Mouse(me) =>
                {
                    let npx = (me.column / 2) as usize;
                    let npy = me.row as usize;
                    if npx >= w || npy >= h
                    {
                        continue;
                    }
                    self.px = npx;
                    self.py = npy;
                    match me.kind
                    {
                        MouseEventKind::Down(MouseButton::Left) =>
                        {
                            if !self.reveal(self.px, self.py)?
                            {
                                return Ok(false);
                            }
                        },
                        MouseEventKind::Down(MouseButton::Right) =>
                        {
                            self.mark_cell(self.px, self.py);
                        },
                        MouseEventKind::Down(MouseButton::Middle) =>
                        {
                            if !self.reveal_area(self.px, self.py)?
                            {
                                return Ok(false);
                            }
                        },
                        _ => { }
                    }
                }
                _ => { }
            }
        }
    }
}