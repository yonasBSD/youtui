use crate::app::component::actionhandler::{
    Action, ComponentEffect, KeyRouter, Scrollable, TextHandler,
};
use crate::app::structures::{
    AlbumSongsList, ListSong, ListSongDisplayableField, ListStatus, Percentage, SongListComponent,
};
use crate::app::ui::action::AppAction;
use crate::app::ui::browser::get_sort_keybinds;
use crate::app::ui::browser::shared_components::{
    get_adjusted_list_column, FilterManager, SortManager,
};
use crate::app::view::{
    BasicConstraint, FilterString, Loadable, SortDirection, SortableTableView, TableFilterCommand,
    TableSortCommand, TableView,
};
use crate::config::keymap::Keymap;
use crate::config::Config;
use anyhow::{bail, Result};
use itertools::Either;
use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::iter::Iterator;
use tracing::warn;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum AlbumSongsInputRouting {
    #[default]
    List,
    Sort,
    Filter,
}

#[derive(Clone)]
pub struct AlbumSongsPanel {
    pub list: AlbumSongsList,
    pub route: AlbumSongsInputRouting,
    pub sort: SortManager,
    pub filter: FilterManager,
    cur_selected: usize,
    pub widget_state: TableState,
}
impl_youtui_component!(AlbumSongsPanel);

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserArtistSongsAction {
    Filter,
    Sort,
    PlaySong,
    PlaySongs,
    PlayAlbum,
    AddSongToPlaylist,
    AddSongsToPlaylist,
    AddAlbumToPlaylist,
}

impl Action for BrowserArtistSongsAction {
    fn context(&self) -> std::borrow::Cow<str> {
        "Artist Songs Panel".into()
    }
    fn describe(&self) -> std::borrow::Cow<str> {
        match &self {
            BrowserArtistSongsAction::PlaySong => "Play song",
            BrowserArtistSongsAction::PlaySongs => "Play songs",
            BrowserArtistSongsAction::PlayAlbum => "Play album",
            BrowserArtistSongsAction::AddSongToPlaylist => "Add song to playlist",
            BrowserArtistSongsAction::AddSongsToPlaylist => "Add songs to playlist",
            BrowserArtistSongsAction::AddAlbumToPlaylist => "Add album to playlist",
            BrowserArtistSongsAction::Sort => "Sort",
            BrowserArtistSongsAction::Filter => "Filter",
        }
        .into()
    }
}
impl AlbumSongsPanel {
    pub fn new() -> AlbumSongsPanel {
        AlbumSongsPanel {
            cur_selected: Default::default(),
            list: Default::default(),
            route: Default::default(),
            sort: SortManager::new(),
            filter: FilterManager::new(),
            widget_state: Default::default(),
        }
    }
    pub fn subcolumns_of_vec() -> [ListSongDisplayableField; 5] {
        [
            ListSongDisplayableField::TrackNo,
            ListSongDisplayableField::Album,
            ListSongDisplayableField::Song,
            ListSongDisplayableField::Duration,
            ListSongDisplayableField::Year,
        ]
    }
    /// Re-apply all sort commands in the stack in the order they were stored.
    pub fn apply_all_sort_commands(&mut self) -> Result<()> {
        for c in self.sort.sort_commands.iter() {
            if !self.get_sortable_columns().contains(&c.column) {
                bail!(format!("Unable to sort column {}", c.column,));
            }
            self.list.sort(
                get_adjusted_list_column(c.column, Self::subcolumns_of_vec())?,
                c.direction,
            );
        }
        Ok(())
    }
    pub fn get_filtered_list_iter(&self) -> Box<dyn Iterator<Item = &ListSong> + '_> {
        Box::new(self.list.get_list_iter().filter(move |ls| {
            // Naive implementation.
            // TODO: Do this in a single pass and optimise.
            self.get_filter_commands()
                .iter()
                .fold(true, |acc, command| {
                    let match_found = command.matches_row(
                        ls,
                        Self::subcolumns_of_vec(),
                        self.get_filterable_columns(),
                    ); // If we find a match for each filter, can display the row.
                    acc && match_found
                })
        }))
    }
    pub fn apply_filter(&mut self) {
        let filter = self.filter.get_text().to_string();
        self.filter.shown = false;
        self.route = AlbumSongsInputRouting::List;
        let cmd = TableFilterCommand::All(crate::app::view::Filter::Contains(
            FilterString::CaseInsensitive(filter),
        ));
        self.filter.filter_commands.push(cmd);
        // Need to match current selected row to length of list.
        // Naive method to count the iterator. Consider making iterator exact sized...
        self.cur_selected = self
            .cur_selected
            .min(self.get_filtered_items().count().saturating_sub(1))
    }
    pub fn clear_filter(&mut self) {
        self.filter.shown = false;
        self.route = AlbumSongsInputRouting::List;
        self.filter.filter_commands.clear();
    }
    fn open_sort(&mut self) {
        self.sort.shown = true;
        self.route = AlbumSongsInputRouting::Sort;
    }
    pub fn toggle_filter(&mut self) {
        let shown = self.filter.shown;
        if !shown {
            // We need to set cur back to 0  and clear text somewhere and I'd prefer to do
            // it at the time of showing, so it cannot be missed.
            self.filter.filter_text.clear();
            self.route = AlbumSongsInputRouting::Filter;
        } else {
            self.route = AlbumSongsInputRouting::List;
        }
        self.filter.shown = !shown;
    }
    pub fn close_sort(&mut self) {
        self.sort.shown = false;
        self.route = AlbumSongsInputRouting::List;
    }
    pub fn handle_pop_sort(&mut self) {
        // If no sortable columns, should we not handle this command?
        self.sort.cur = 0;
        self.open_sort();
    }
    pub fn handle_clear_sort(&mut self) {
        self.close_sort();
        self.clear_sort_commands();
    }
    pub fn handle_sort_cur_asc(&mut self) {
        // TODO: Better error handling
        let Some(column) = self.get_sortable_columns().get(self.sort.cur) else {
            warn!("Tried to index sortable columns but was out of range");
            return;
        };
        if let Err(e) = self.push_sort_command(TableSortCommand {
            column: *column,
            direction: SortDirection::Asc,
        }) {
            warn!("Tried to sort a column that is not sortable - error {e}")
        };
        self.close_sort();
    }
    pub fn handle_sort_cur_desc(&mut self) {
        // TODO: Better error handling
        let Some(column) = self.get_sortable_columns().get(self.sort.cur) else {
            warn!("Tried to index sortable columns but was out of range");
            return;
        };
        if let Err(e) = self.push_sort_command(TableSortCommand {
            column: *column,
            direction: SortDirection::Desc,
        }) {
            warn!("Tried to sort a column that is not sortable - error {e}")
        };
        self.close_sort();
    }
    pub fn handle_songs_found(&mut self) {
        self.list.clear();
        // XXX: Consider clearing sort params here, so that we don't need to sort all
        // the incoming songs. Performance seems OK for now. XXX: Consider also
        // clearing filter params here.
        self.cur_selected = 0;
        self.list.state = ListStatus::InProgress;
    }
}
impl SongListComponent for AlbumSongsPanel {
    fn get_song_from_idx(&self, idx: usize) -> Option<&crate::app::structures::ListSong> {
        self.get_filtered_list_iter().nth(idx)
    }
}
impl TextHandler for AlbumSongsPanel {
    fn get_text(&self) -> &str {
        self.filter.get_text()
    }
    fn replace_text(&mut self, text: impl Into<String>) {
        self.filter.replace_text(text)
    }
    fn is_text_handling(&self) -> bool {
        self.route == AlbumSongsInputRouting::Filter
    }
    fn clear_text(&mut self) -> bool {
        self.filter.clear_text()
    }
    fn handle_text_event_impl(
        &mut self,
        event: &crossterm::event::Event,
    ) -> Option<ComponentEffect<Self>> {
        self.filter
            .handle_text_event_impl(event)
            .map(|effect| effect.map(|this: &mut AlbumSongsPanel| &mut this.filter))
    }
}

impl KeyRouter<AppAction> for AlbumSongsPanel {
    fn get_all_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        std::iter::once(&config.keybinds.browser_artist_songs)
    }
    fn get_active_keybinds<'a>(
        &self,
        config: &'a Config,
    ) -> impl Iterator<Item = &'a Keymap<AppAction>> + 'a {
        match self.route {
            AlbumSongsInputRouting::List => {
                Either::Left(std::iter::once(&config.keybinds.browser_artist_songs))
            }
            AlbumSongsInputRouting::Filter => {
                Either::Left(std::iter::once(&config.keybinds.filter))
            }
            AlbumSongsInputRouting::Sort => Either::Right(get_sort_keybinds(config)),
        }
    }
}

// Is this still relevant?
impl Loadable for AlbumSongsPanel {
    fn is_loading(&self) -> bool {
        matches!(self.list.state, crate::app::structures::ListStatus::Loading)
    }
}
impl Scrollable for AlbumSongsPanel {
    fn increment_list(&mut self, amount: isize) {
        if self.sort.shown {
            self.sort.cur = self
                .sort
                .cur
                .saturating_add_signed(amount)
                .min(self.get_sortable_columns().len().saturating_sub(1));
        } else {
            // Naive check using iterator - consider using exact size iterator
            self.cur_selected = self
                .cur_selected
                .saturating_add_signed(amount)
                .min(self.get_filtered_items().count().saturating_sub(1))
        }
    }
    fn is_scrollable(&self) -> bool {
        !self.filter.shown
    }
}

impl TableView for AlbumSongsPanel {
    fn get_selected_item(&self) -> usize {
        self.cur_selected
    }
    fn get_state(&self) -> ratatui::widgets::TableState {
        self.widget_state.clone()
    }
    fn get_title(&self) -> Cow<str> {
        match self.list.state {
            ListStatus::New => "Songs".into(),
            ListStatus::Loading => "Songs - loading".into(),
            ListStatus::InProgress => format!(
                "Songs - {} results - loading",
                self.list.get_list_iter().len()
            )
            .into(),
            ListStatus::Loaded => {
                format!("Songs - {} results", self.list.get_list_iter().len()).into()
            }
            ListStatus::Error => "Songs - Error receieved".into(),
        }
    }
    fn get_layout(&self) -> &[BasicConstraint] {
        &[
            BasicConstraint::Length(4),
            BasicConstraint::Percentage(Percentage(50)),
            BasicConstraint::Percentage(Percentage(50)),
            BasicConstraint::Length(10),
            BasicConstraint::Length(5),
        ]
    }
    fn get_items(
        &self,
    ) -> Box<dyn ExactSizeIterator<Item = impl Iterator<Item = Cow<'_, str>> + '_> + '_> {
        let b = self
            .list
            .get_list_iter()
            .map(|ls| ls.get_fields(Self::subcolumns_of_vec()).into_iter());
        Box::new(b)
    }
    fn get_headings(&self) -> Box<(dyn Iterator<Item = &'static str> + 'static)> {
        Box::new(["#", "Album", "Song", "Duration", "Year"].into_iter())
    }
    fn get_highlighted_row(&self) -> Option<usize> {
        None
    }
}
impl SortableTableView for AlbumSongsPanel {
    fn get_sortable_columns(&self) -> &[usize] {
        &[1, 4]
    }
    fn push_sort_command(&mut self, sort_command: TableSortCommand) -> Result<()> {
        // TODO: Maintain a view only struct, for easier rendering of this.
        if !self.get_sortable_columns().contains(&sort_command.column) {
            bail!(format!("Unable to sort column {}", sort_command.column,));
        }
        // Map the column of ArtistAlbums to a column of List and sort
        self.list.sort(
            get_adjusted_list_column(sort_command.column, Self::subcolumns_of_vec())?,
            sort_command.direction,
        );
        // Remove commands that already exist for the same column, as this new command
        // will trump the old ones. Slightly naive - loops the whole vec, could
        // short circuit.
        self.sort
            .sort_commands
            .retain(|cmd| cmd.column != sort_command.column);
        self.sort.sort_commands.push(sort_command);
        Ok(())
    }
    fn clear_sort_commands(&mut self) {
        self.sort.sort_commands.clear();
    }
    fn get_sort_commands(&self) -> &[TableSortCommand] {
        &self.sort.sort_commands
    }
    fn get_filtered_items(
        &self,
    ) -> Box<dyn Iterator<Item = impl Iterator<Item = Cow<'_, str>> + '_> + '_> {
        // We are doing a lot here every draw cycle!
        Box::new(
            self.get_filtered_list_iter()
                .map(|ls| ls.get_fields(Self::subcolumns_of_vec()).into_iter()),
        )
    }
    fn get_filterable_columns(&self) -> &[usize] {
        &[1, 2, 4]
    }
    fn get_filter_commands(&self) -> &[TableFilterCommand] {
        &self.filter.filter_commands
    }
    fn push_filter_command(&mut self, filter_command: TableFilterCommand) {
        self.filter.filter_commands.push(filter_command)
    }
    fn clear_filter_commands(&mut self) {
        self.filter.filter_commands.clear()
    }
    fn get_sort_popup_cur(&self) -> usize {
        self.sort.cur
    }
    fn get_sort_popup_state(&self) -> ratatui::widgets::ListState {
        self.sort.state.clone()
    }
}
