use std::io::{stdout, Write};
use rand::Rng;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{read, Event, KeyCode, EnableMouseCapture, DisableMouseCapture, MouseEventKind, MouseButton};
use crossterm::cursor::{MoveTo, Show, Hide};
use crossterm::style::{SetForegroundColor, Color, SetBackgroundColor};
use crossterm::{Result, execute, queue};

const MINE: u32 = 16;
const UNKNOWN: u32 = MINE << 1;
const MARKER: u32 = UNKNOWN << 1;
const NUMBER_MASK: u32 = MINE-1;

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

struct Minesweeper
{
    grid: [[u32; h]; w],
    generated: bool,
    px: usize,
    py: usize,
    mine_count: u32,
    flag_count: i32
}

impl Minesweeper
{
    fn new() -> Self
    {
        Self
        {
            mine_count: 99,
            grid: [[UNKNOWN; h]; w],

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

                if (self.grid[x][y] & UNKNOWN) != 0
                {
                    if (self.grid[x][y] & MARKER) != 0
                    {
                        queue!(stdout, SetForegroundColor(Color::Red))?;
                        print!("P");
                    }
                    else
                    {
                        queue!(stdout, SetForegroundColor(Color::Reset))?;
                        print!("■");
                    }
                }
                else if (self.grid[x][y] & MINE) != 0
                {
                    queue!(stdout, SetForegroundColor(Color::Red))?;
                    print!("*");
                }
                else if (self.grid[x][y] & NUMBER_MASK) == 0
                {
                    print!(" ");
                }
                else
                {
                    queue!(stdout, SetForegroundColor(Color::Cyan))?;
                    print!("{}", self.grid[x][y] & NUMBER_MASK);
                }
                queue!(stdout, SetBackgroundColor(Color::Reset))?;
                print!(" ");
            }
        }
        Ok(())
    }

    fn count_neighbors(&self, x: usize, y: usize, mask: u32) -> u32
    {
        let mut n = 0;
        for nx in x.checked_sub(1).unwrap_or(0)..=x+1
        {
            for ny in y.checked_sub(1).unwrap_or(0)..=y+1
            {
                if nx < w && ny < h && (self.grid[nx][ny] & mask) > 0
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
                if (self.grid[x][y] & MINE) == 0 &&
                ((x as isize - self.px as isize).abs() >  1 || (y as isize - self.py as isize).abs() >  1)
                {
                    break;
                }
            }

            self.grid[x][y] = MINE | UNKNOWN;
        }
        
        for x in 0..w
        {
            for y in 0..h
            {
                let n = self.count_neighbors(x, y, MINE);
                self.grid[x][y] |= n;
            }
        }
        self.generated = true;
    }

    fn mark_cell(&mut self, px: usize, py: usize)
    {
        if (self.grid[px][py] & UNKNOWN) != 0
        {
            self.grid[px][py] ^= MARKER;
            if (self.grid[px][py] & MARKER) != 0
            {
                self.flag_count += 1;
            }
            else
            {
                self.flag_count -= 1;
            }
        }
    }

    fn reveal_area(&mut self, x: usize, y: usize) -> Result<bool>
    {
        let markers = self.count_neighbors(x, y, MARKER);
        let nb = self.grid[x][y] & NUMBER_MASK;
        if nb == markers
        {
            let mut res = true;
            for nx in x.checked_sub(1).unwrap_or(0)..=x+1
            {
                for ny in y.checked_sub(1).unwrap_or(0)..=y+1
                {
                    if nx < w && ny < h && (self.grid[nx][ny] & MARKER) == 0
                    {
                        res &= self.reveal(nx, ny)?
                    }
                }
            }
            return Ok(res)
        }
        return Ok(true)
    }

    fn reveal(&mut self, px: usize, py: usize) -> Result<bool>
    {
        if !self.generated
        {
            self.generate_grid();
        }

        if (self.grid[px][py] & UNKNOWN) == 0
        {
            return Ok(true);
        }
        
        if (self.grid[px][py] & MARKER) != 0
        {
            return Ok(true);
        }

        self.grid[px][py] = self.grid[px][py] & !UNKNOWN & !MARKER;
        if (self.grid[px][py] & MINE) != 0
        {
            return Ok(false)
        }
        if (self.grid[px][py] & NUMBER_MASK) == 0
        {
            // Propagate reveal
            for nx in px.checked_sub(1).unwrap_or(0)..=px+1
            {
                for ny in py.checked_sub(1).unwrap_or(0)..=py+1
                {
                    if nx < w && ny < h
                    {
                        self.reveal(nx, ny)?;
                    }
                }
            }
        }
        return Ok(true)
    }

    fn run_game(mut self) -> Result<bool>
    {
        loop
        {
            let mut stdout = stdout();
            self.print_grid()?;
            queue!(stdout, MoveTo(0, h as u16))?;
            print!("{}   ", self.mine_count as i32 - self.flag_count);
            stdout.flush()?;
            
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