using System;
using System.Collections.Generic;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Reqnroll;

namespace FrigoTab.AcceptanceTests.Bindings {

    [Binding]
    public sealed class GridLayoutSteps {

        private readonly IList<LayoutMonitor> monitors = new List<LayoutMonitor>();
        private readonly IList<LayoutWindow> windows = new List<LayoutWindow>();
        private readonly IDictionary<string, LayoutMonitor> monitorsById = new Dictionary<string, LayoutMonitor>(StringComparer.Ordinal);
        private readonly IDictionary<string, LayoutWindow> windowsById = new Dictionary<string, LayoutWindow>(StringComparer.Ordinal);
        private IDictionary<string, ScreenRectangle> tiles;

        [Given(@"monitor ""([^"" ]+)"" has bounds (-?\d+),(-?\d+),(-?\d+),(-?\d+) and working area (-?\d+),(-?\d+),(-?\d+),(-?\d+)")]
        public void GivenMonitorHasBoundsAndWorkingArea (
            string id,
            int boundsX,
            int boundsY,
            int boundsWidth,
            int boundsHeight,
            int workingX,
            int workingY,
            int workingWidth,
            int workingHeight) {
            var monitor = new LayoutMonitor(
                id,
                new ScreenRectangle(boundsX, boundsY, boundsWidth, boundsHeight),
                new ScreenRectangle(workingX, workingY, workingWidth, workingHeight));
            monitors.Add(monitor);
            monitorsById[id] = monitor;
        }

        [Given(@"layout window ""([^"" ]+)"" is on monitor ""([^"" ]+)"" at (-?\d+),(-?\d+) with size (-?\d+),(-?\d+)")]
        public void GivenLayoutWindowIsOnMonitor (
            string id,
            string monitorId,
            int x,
            int y,
            int width,
            int height) {
            var window = new LayoutWindow(id, monitorId, new ScreenRectangle(x, y, width, height));
            windows.Add(window);
            windowsById[id] = window;
        }

        [When(@"I arrange the grid layout")]
        public void WhenIArrangeTheGridLayout () {
            tiles = new GridLayout().Arrange(windows, monitors);
        }

        [Then(@"(\d+) tiles are produced")]
        public void ThenTilesAreProduced (int count) {
            RequireArranged();
            Assert.AreEqual(count, tiles.Count);
        }

        [Then(@"monitor ""([^"" ]+)"" has (\d+) columns and (\d+) rows of tiles")]
        public void ThenMonitorHasColumnsAndRowsOfTiles (string monitorId, int columns, int rows) {
            RequireArranged();
            var xPositions = new HashSet<int>();
            var yPositions = new HashSet<int>();
            foreach( KeyValuePair<string, ScreenRectangle> tile in tiles ) {
                LayoutWindow window;
                if( !windowsById.TryGetValue(tile.Key, out window) || window.MonitorId != monitorId ) {
                    continue;
                }
                xPositions.Add(tile.Value.X);
                yPositions.Add(tile.Value.Y);
            }

            Assert.AreEqual(columns, xPositions.Count, "Unexpected number of grid columns.");
            Assert.AreEqual(rows, yPositions.Count, "Unexpected number of grid rows.");
        }

        [Then(@"every produced tile is inside monitor ""([^"" ]+)"" working area")]
        public void ThenEveryProducedTileIsInsideMonitorWorkingArea (string monitorId) {
            RequireArranged();
            LayoutMonitor monitor = GetMonitor(monitorId);
            foreach( KeyValuePair<string, ScreenRectangle> tile in tiles ) {
                LayoutWindow window;
                if( !windowsById.TryGetValue(tile.Key, out window) || window.MonitorId != monitorId ) {
                    continue;
                }
                AssertInside(tile.Value, monitor.WorkingArea, tile.Key);
            }
        }

        [Then(@"tile ""([^"" ]+)"" is inside monitor ""([^"" ]+)"" working area")]
        public void ThenTileIsInsideMonitorWorkingArea (string windowId, string monitorId) {
            RequireArranged();
            ScreenRectangle tile = GetTile(windowId);
            LayoutMonitor monitor = GetMonitor(monitorId);
            Assert.AreEqual(monitorId, windowsById[windowId].MonitorId);
            AssertInside(tile, monitor.WorkingArea, windowId);
        }

        [Then(@"tile ""([^"" ]+)"" preserves its source aspect ratio within ([0-9.]+)")]
        public void ThenTilePreservesSourceAspectRatioWithin (string windowId, double tolerance) {
            RequireArranged();
            ScreenRectangle source = windowsById[windowId].Bounds;
            ScreenRectangle tile = GetTile(windowId);
            double sourceRatio = source.Width / (double) source.Height;
            double tileRatio = tile.Width / (double) tile.Height;
            Assert.IsTrue(
                Math.Abs(sourceRatio - tileRatio) <= tolerance,
                "Source ratio " + sourceRatio + " and tile ratio " + tileRatio + " differ by more than " + tolerance + ".");
        }

        [Then(@"tile ""([^"" ]+)"" has a positive margin inside monitor ""([^"" ]+)""")]
        public void ThenTileHasPositiveMarginInsideMonitor (string windowId, string monitorId) {
            RequireArranged();
            ScreenRectangle tile = GetTile(windowId);
            ScreenRectangle area = GetMonitor(monitorId).WorkingArea;
            Assert.IsTrue(tile.X > area.X, "The tile should have a left margin.");
            Assert.IsTrue(tile.Y > area.Y, "The tile should have a top margin.");
            Assert.IsTrue(tile.Right < area.Right, "The tile should have a right margin.");
            Assert.IsTrue(tile.Bottom < area.Bottom, "The tile should have a bottom margin.");
        }

        [Then(@"tile ""([^"" ]+)"" has a negative screen origin")]
        public void ThenTileHasNegativeScreenOrigin (string windowId) {
            RequireArranged();
            ScreenRectangle tile = GetTile(windowId);
            Assert.IsTrue(tile.X < 0 || tile.Y < 0, "The tile should preserve the negative monitor origin.");
        }

        [Then(@"no tile is produced for ""([^"" ]+)""")]
        public void ThenNoTileIsProducedFor (string windowId) {
            RequireArranged();
            Assert.IsFalse(tiles.ContainsKey(windowId), "An invalid source rectangle should be ignored.");
        }

        private void RequireArranged () {
            Assert.IsNotNull(tiles, "The grid layout has not been arranged yet.");
        }

        private LayoutMonitor GetMonitor (string id) {
            LayoutMonitor monitor;
            Assert.IsTrue(monitorsById.TryGetValue(id, out monitor), "Unknown monitor: " + id);
            return monitor;
        }

        private ScreenRectangle GetTile (string id) {
            ScreenRectangle tile;
            Assert.IsTrue(tiles.TryGetValue(id, out tile), "No tile was produced for: " + id);
            return tile;
        }

        private static void AssertInside (ScreenRectangle tile, ScreenRectangle area, string id) {
            Assert.IsFalse(tile.IsEmpty, "Tile " + id + " should have positive dimensions.");
            Assert.IsTrue(tile.X >= area.X, "Tile " + id + " extends left of the working area.");
            Assert.IsTrue(tile.Y >= area.Y, "Tile " + id + " extends above the working area.");
            Assert.IsTrue(tile.Right <= area.Right, "Tile " + id + " extends right of the working area.");
            Assert.IsTrue(tile.Bottom <= area.Bottom, "Tile " + id + " extends below the working area.");
        }

    }

}
