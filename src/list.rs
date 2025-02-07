use crate::{float::floating_window, running_command::Command, state::AppState};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ego_tree::{tree, NodeId};
use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListState},
    Frame,
};

#[derive(Clone)]
struct ListNode {
    name: &'static str,
    command: Command,
}

/// This is a data structure that has everything necessary to draw and manage a menu of commands
pub struct CustomList {
    /// The tree data structure, to represent regular items
    /// and "directories"
    inner_tree: ego_tree::Tree<ListNode>,
    /// This stack keeps track of our "current dirrectory". You can think of it as `pwd`. but not
    /// just the current directory, all paths that took us here, so we can "cd .."
    visit_stack: Vec<NodeId>,
    /// This is the state asociated with the list widget, used to display the selection in the
    /// widget
    list_state: ListState,
    /// This stores the preview windows state. If it is None, it will not be displayed.
    /// If it is Some, we show it with the content of the selected item
    preview_window_state: Option<PreviewWindowState>,
    // This stores the current search query
    filter_query: String,
    // This stores the filtered tree
    filtered_items: Vec<ListNode>,
}

/// This struct stores the preview window state
struct PreviewWindowState {
    /// The text inside the window
    text: Vec<String>,
    /// The current line scroll
    scroll: usize,
}

impl PreviewWindowState {
    /// Create a new PreviewWindowState
    pub fn new(text: Vec<String>) -> Self {
        Self { text, scroll: 0 }
    }
}

impl CustomList {
    pub fn new() -> Self {
        // When a function call ends with an exclamation mark, it means it's a macro, like in this
        // case the tree! macro expands to `ego-tree::tree` data structure
        let tree = tree!(ListNode {
            name: "root",
            command: Command::None,
        } => {
            ListNode {
                name: "System Setup",
                command: Command::None,
            } => {
                ListNode {
                    name: "Build Prerequisites",
                    command: Command::LocalFile("system-setup/1-compile-setup.sh"),
                },
                ListNode {
                    name: "Gaming Dependencies",
                    command: Command::LocalFile("system-setup/2-gaming-setup.sh"),
                },
                ListNode {
                    name: "Global Theme",
                    command: Command::LocalFile("system-setup/3-global-theme.sh"),
                },
            },
            ListNode {
                name: "Security",
                command: Command::None
            } => {
                ListNode {
                    name: "Firewall Baselines (CTT)",
                    command: Command::LocalFile("security/firewall-baselines.sh"),
                }
            },
            ListNode {
                name: "Applications Setup",
                command: Command::None
            } => {
                ListNode {
                    name: "Alacritty Setup",
                    command: Command::LocalFile("applications-setup/alacritty-setup.sh"),
                },
                ListNode {
                    name: "Bash Prompt Setup",
                    command: Command::Raw("bash -c \"$(curl -s https://raw.githubusercontent.com/ChrisTitusTech/mybash/main/setup.sh)\""),
                },
                ListNode {
                    name: "Kitty Setup",
                    command: Command::LocalFile("applications-setup/kitty-setup.sh")
                },
                ListNode {
                    name: "Neovim Setup",
                    command: Command::Raw("bash -c \"$(curl -s https://raw.githubusercontent.com/ChrisTitusTech/neovim/main/setup.sh)\""),
                },
                ListNode {
                    name: "Rofi Setup",
                    command: Command::LocalFile("applications-setup/rofi-setup.sh"),
                },
            },
            ListNode {
                name: "Full System Update",
                command: Command::LocalFile("system-update.sh"),
            },
        });
        // We don't get a reference, but rather an id, because references are siginficantly more
        // paintfull to manage
        let root_id = tree.root().id();
        Self {
            inner_tree: tree,
            visit_stack: vec![root_id],
            list_state: ListState::default().with_selected(Some(0)),
            // By default the PreviewWindowState is set to None, so it is not being shown
            preview_window_state: None,
            filter_query: String::new(),
            filtered_items: vec![],
        }
    }

    /// Draw our custom widget to the frame
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, filter: String, state: &AppState) {
        self.filter(filter);

        let item_list: Vec<Line> = if self.filter_query.is_empty() {
            let mut items: Vec<Line> = vec![];
            // If we are not at the root of our filesystem tree, we need to add `..` path, to be able
            // to go up the tree
            // icons:   
            if !self.at_root() {
                items.push(
                    Line::from(format!("{}  ..", state.theme.dir_icon))
                        .style(state.theme.dir_color),
                );
            }

            // Get the last element in the `visit_stack` vec
            let curr = self
                .inner_tree
                .get(*self.visit_stack.last().unwrap())
                .unwrap();

            // Iterate through all the children
            for node in curr.children() {
                // The difference between a "directory" and a "command" is simple: if it has children,
                // it's a directory and will be handled as such
                if node.has_children() {
                    items.push(
                        Line::from(format!("{}  {}", state.theme.dir_icon, node.value().name))
                            .style(state.theme.dir_color),
                    );
                } else {
                    items.push(
                        Line::from(format!("{}  {}", state.theme.cmd_icon, node.value().name))
                            .style(state.theme.cmd_color),
                    );
                }
            }
            items
        } else {
            let mut sorted_items = self.filtered_items.clone();
            sorted_items.sort_by(|a, b| a.name.cmp(b.name));
            sorted_items
                .iter()
                .map(|node| {
                    Line::from(format!("{}  {}", state.theme.cmd_icon, node.name))
                        .style(state.theme.cmd_color)
                })
                .collect()
        };

        // create the normal list widget containing only item in our "working directory" / tree
        // node
        let list = List::new(item_list)
            .highlight_style(Style::default().reversed())
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Linux Toolbox - {}",
                chrono::Local::now().format("%Y-%m-%d")
            )))
            .scroll_padding(1);

        // Render it
        frame.render_stateful_widget(list, area, &mut self.list_state);

        // Draw the preview window if it's active
        if let Some(pw_state) = &self.preview_window_state {
            // Set the window to be floating
            let floating_area = floating_window(area);

            // Draw the preview windows lines
            let lines: Vec<Line> = pw_state
                .text
                .iter()
                .skip(pw_state.scroll)
                .take(floating_area.height as usize)
                .map(|line| Line::from(line.as_str()))
                .collect();

            // Create list widget
            let list = List::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Action preview"),
                )
                .highlight_style(Style::default().reversed());

            // Finally render the preview window
            frame.render_widget(list, floating_area);
        }
    }

    pub fn filter(&mut self, query: String) {
        self.filter_query.clone_from(&query);
        self.filtered_items.clear();

        let query_lower = query.to_lowercase();
        let mut stack = vec![self.inner_tree.root().id()];

        while let Some(node_id) = stack.pop() {
            let node = self.inner_tree.get(node_id).unwrap();

            if node.value().name.to_lowercase().contains(&query_lower) && !node.has_children() {
                self.filtered_items.push(node.value().clone());
            }

            for child in node.children() {
                stack.push(child.id());
            }
        }
    }

    /// Resets the selection to the first item
    pub fn reset_selection(&mut self) {
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Handle key events, we are only interested in `Press` and `Repeat` events
    pub fn handle_key(&mut self, event: KeyEvent, state: &AppState) -> Option<Command> {
        if event.kind == KeyEventKind::Release {
            return None;
        }
        match event.code {
            // Damm you Up arrow, use vim lol
            KeyCode::Char('j') | KeyCode::Down => {
                // If the preview window is active, scroll down and consume the scroll action,
                // so the scroll does not happen in the main window as well
                if self.preview_window_state.is_some() {
                    self.scroll_preview_window_down();
                    return None;
                }

                self.try_scroll_down();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // If the preview window is active, scroll up and consume the scroll action,
                // so the scroll does not happen in the main window as well
                if self.preview_window_state.is_some() {
                    self.scroll_preview_window_up();
                    return None;
                }

                self.try_scroll_up();
                None
            }
            // The 'p' key toggles the preview on and off
            KeyCode::Char('p') => {
                self.toggle_preview_window(state);
                None
            }

            KeyCode::Enter => { 
                if self.preview_window_state.is_none() {
                    self.handle_enter()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn toggle_preview_window(&mut self, state: &AppState) {
        // If the preview window is active, disable it
        if self.preview_window_state.is_some() {
            self.preview_window_state = None;
        } else {
            // If the preview windows is not active, show it

            // Get the selected command
            if let Some(selected_command) = self.get_selected_command() {
                let lines = match selected_command {
                    Command::Raw(cmd) => {
                        // Reconstruct the line breaks and file formatting after the
                        // 'include_str!()' call in the node
                        cmd.lines().map(|line| line.to_string()).collect()
                    }
                    Command::LocalFile(file_path) => {
                        let mut full_path = state.temp_path.clone();
                        full_path.push(file_path);
                        let file_contents = std::fs::read_to_string(&full_path)
                            .map_err(|_| format!("File not found: {:?}", &full_path))
                            .unwrap();
                        file_contents.lines().map(|line| line.to_string()).collect()
                    }
                    // If command is a folder, we don't display a preview
                    Command::None => return,
                };

                // Show the preview window with the text lines
                self.preview_window_state = Some(PreviewWindowState::new(lines));
            }
        }
    }

    fn try_scroll_up(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected > 0 {
                self.list_state.select(Some(selected.saturating_sub(1)));
            }
        }
    }

    fn try_scroll_down(&mut self) {
        let count = if self.filter_query.is_empty() {
            let curr = self
                .inner_tree
                .get(*self.visit_stack.last().unwrap())
                .unwrap();
            curr.children().count()
        } else {
            self.filtered_items.len()
        };

        if let Some(curr_selection) = self.list_state.selected() {
            if self.at_root() {
                self.list_state
                    .select(Some((curr_selection + 1).min(count - 1)));
            } else {
                // When we are not at the root, we have to account for 1 more "virtual" node, `..`. So
                // the count is 1 bigger (select is 0 based, because it's an index)
                self.list_state
                    .select(Some((curr_selection + 1).min(count)));
            }
        }
    }

    /// Scroll the preview window down
    fn scroll_preview_window_down(&mut self) {
        if let Some(pw_state) = &mut self.preview_window_state {
            if pw_state.scroll + 1 < pw_state.text.len() {
                pw_state.scroll += 1;
            }
        }
    }

    /// Scroll the preview window up
    fn scroll_preview_window_up(&mut self) {
        if let Some(pw_state) = &mut self.preview_window_state {
            if pw_state.scroll > 0 {
                pw_state.scroll = pw_state.scroll.saturating_sub(1);
            }
        }
    }

    /// This method returns the currently selected command, or None if no command is selected.
    /// It was extracted from the 'handle_enter()'
    ///
    /// This could probably be integrated into the 'handle_enter()' method to avoid code
    /// duplication, but I don't want to make too major changes to the codebase.
    fn get_selected_command(&self) -> Option<Command> {
        let selected = self.list_state.selected().unwrap();

        if self.filter_query.is_empty() {
            // No filter query, use the regular tree navigation
            let curr = self
                .inner_tree
                .get(*self.visit_stack.last().unwrap())
                .unwrap();

            // If we are not at the root and the first item is selected, it's the `..` item
            if !self.at_root() && selected == 0 {
                return None;
            }

            for (mut idx, node) in curr.children().enumerate() {
                if !self.at_root() {
                    idx += 1;
                }
                if idx == selected {
                    return Some(node.value().command.clone());
                }
            }
        } else {
            // Filter query is active, use the filtered items
            if let Some(filtered_node) = self.filtered_items.get(selected) {
                return Some(filtered_node.command.clone());
            }
        }
        None
    }

    /// Handles the <Enter> key. This key can do 3 things:
    /// - Run a command, if it is the currently selected item,
    /// - Go up a directory
    /// - Go down into a directory
    ///
    /// Returns `Some(command)` when command is selected, othervise we returns `None`
    fn handle_enter(&mut self) -> Option<Command> {
        // Ensure an item is selected if none is selected
        if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }

        // Get the selected index
        let selected = self.list_state.selected().unwrap();

        if self.filter_query.is_empty() {
            // No filter query, use the regular tree navigation
            let curr = self
                .inner_tree
                .get(*self.visit_stack.last().unwrap())
                .unwrap();

            // if we are not at the root, and the first element is selected,
            // we can be sure it's '..', so we go up the directory
            if !self.at_root() && selected == 0 {
                self.visit_stack.pop();
                self.list_state.select(Some(0));
                return None;
            }

            for (mut idx, node) in curr.children().enumerate() {
                // at this point, we know that we are not on the .. item, and our indexes of the items never had ..
                // item. so to balance it out, in case the selection index contains .., se add 1 to our node index
                if !self.at_root() {
                    idx += 1;
                }
                if idx == selected {
                    if node.has_children() {
                        self.visit_stack.push(node.id());
                        self.list_state.select(Some(0));
                        return None;
                    } else {
                        return Some(node.value().command.clone());
                    }
                }
            }
        } else {
            // Filter query is active, use the filtered items
            if let Some(filtered_node) = self.filtered_items.get(selected) {
                return Some(filtered_node.command.clone());
            }
        }
        None
    }

    /// Checks weather the current tree node is the root node (can we go up the tree or no)
    /// Returns `true` if we can't go up the tree (we are at the tree root)
    /// else returns `false`
    fn at_root(&self) -> bool {
        self.visit_stack.len() == 1
    }
}
