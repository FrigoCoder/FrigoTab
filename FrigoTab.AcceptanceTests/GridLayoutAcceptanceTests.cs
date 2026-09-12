using System;
using System.Collections.Generic;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("CurrentFeature")]
    [TestCategory("Regression")]
    public sealed class GridLayoutAcceptanceTests {

        [TestMethod]
        public void T20260912T090300Z_031_FourWindowsUseANearSquareGrid () {
            LayoutMonitor primary = Monitor("primary", 0, 0, 1920, 1080, 0, 0, 1920, 1040);
            var windows = new List<LayoutWindow> {
                Window("one", "primary", 800, 600),
                Window("two", "primary", 800, 600),
                Window("three", "primary", 800, 600),
                Window("four", "primary", 800, 600)
            };

            IDictionary<string, ScreenRectangle> tiles = Arrange(windows, primary);

            Assert.AreEqual(4, tiles.Count);
            Assert.AreEqual(2, CountDistinctX(tiles, windows, "primary"));
            Assert.AreEqual(2, CountDistinctY(tiles, windows, "primary"));
            AssertAllInside(tiles, windows, primary);
        }

        [TestMethod]
        public void T20260912T090300Z_032_WideSourceWindowKeepsItsAspectRatio () {
            LayoutMonitor primary = Monitor("primary", 0, 0, 1000, 800, 0, 0, 1000, 800);
            var windows = new List<LayoutWindow> {
                Window("wide", "primary", 1600, 900)
            };

            IDictionary<string, ScreenRectangle> tiles = Arrange(windows, primary);
            ScreenRectangle tile = GetTile(tiles, "wide");
            double sourceRatio = windows[0].Bounds.Width / (double) windows[0].Bounds.Height;
            double tileRatio = tile.Width / (double) tile.Height;

            Assert.IsTrue(
                Math.Abs(sourceRatio - tileRatio) <= 0.02,
                "Source ratio " + sourceRatio + " and tile ratio " + tileRatio + " differ by more than 0.02.");
            AssertInside(tile, primary.WorkingArea, "wide");
        }

        [TestMethod]
        public void T20260912T090300Z_033_TilesRetainAMarginFromTheMonitorEdges () {
            LayoutMonitor primary = Monitor("primary", 0, 0, 1000, 1000, 0, 0, 1000, 1000);
            var windows = new List<LayoutWindow> {
                Window("square", "primary", 1000, 1000)
            };

            ScreenRectangle tile = GetTile(Arrange(windows, primary), "square");
            ScreenRectangle area = primary.WorkingArea;

            Assert.IsTrue(tile.X > area.X, "The tile should have a left margin.");
            Assert.IsTrue(tile.Y > area.Y, "The tile should have a top margin.");
            Assert.IsTrue(tile.Right < area.Right, "The tile should have a right margin.");
            Assert.IsTrue(tile.Bottom < area.Bottom, "The tile should have a bottom margin.");
        }

        [TestMethod]
        public void T20260912T090300Z_034_EachMonitorReceivesItsOwnIndependentGrid () {
            LayoutMonitor left = Monitor("left", -1920, 0, 1920, 1080, -1920, 0, 1920, 1040);
            LayoutMonitor primary = Monitor("primary", 0, 0, 1920, 1080, 0, 0, 1920, 1040);
            var windows = new List<LayoutWindow> {
                new LayoutWindow("leftApp", "left", new ScreenRectangle(-1800, 100, 800, 600)),
                new LayoutWindow("rightApp", "primary", new ScreenRectangle(100, 100, 800, 600))
            };

            IDictionary<string, ScreenRectangle> tiles = Arrange(windows, left, primary);
            ScreenRectangle leftTile = GetTile(tiles, "leftApp");
            ScreenRectangle rightTile = GetTile(tiles, "rightApp");

            Assert.IsTrue(leftTile.X < 0 || leftTile.Y < 0, "The left tile should preserve the negative screen origin.");
            AssertInside(leftTile, left.WorkingArea, "leftApp");
            AssertInside(rightTile, primary.WorkingArea, "rightApp");
        }

        [TestMethod]
        public void T20260912T090300Z_035_InvalidSourceDimensionsAreIgnored () {
            LayoutMonitor primary = Monitor("primary", 0, 0, 1920, 1080, 0, 0, 1920, 1040);
            var windows = new List<LayoutWindow> {
                Window("zeroWidth", "primary", 0, 600),
                Window("negativeHeight", "primary", 800, -1)
            };

            IDictionary<string, ScreenRectangle> tiles = Arrange(windows, primary);

            Assert.AreEqual(0, tiles.Count);
            Assert.IsFalse(tiles.ContainsKey("zeroWidth"), "An invalid source rectangle should be ignored.");
            Assert.IsFalse(tiles.ContainsKey("negativeHeight"), "An invalid source rectangle should be ignored.");
        }

        private static LayoutMonitor Monitor (
            string id,
            int boundsX,
            int boundsY,
            int boundsWidth,
            int boundsHeight,
            int workingX,
            int workingY,
            int workingWidth,
            int workingHeight) {
            return new LayoutMonitor(
                id,
                new ScreenRectangle(boundsX, boundsY, boundsWidth, boundsHeight),
                new ScreenRectangle(workingX, workingY, workingWidth, workingHeight));
        }

        private static LayoutWindow Window (string id, string monitorId, int width, int height) {
            return new LayoutWindow(id, monitorId, new ScreenRectangle(0, 0, width, height));
        }

        private static IDictionary<string, ScreenRectangle> Arrange (
            IList<LayoutWindow> windows,
            params LayoutMonitor[] monitors) {
            return new GridLayout().Arrange(windows, new List<LayoutMonitor>(monitors));
        }

        private static int CountDistinctX (
            IDictionary<string, ScreenRectangle> tiles,
            IEnumerable<LayoutWindow> windows,
            string monitorId) {
            var values = new HashSet<int>();
            foreach( LayoutWindow window in windows ) {
                if( window.MonitorId == monitorId ) {
                    values.Add(GetTile(tiles, window.Id).X);
                }
            }
            return values.Count;
        }

        private static int CountDistinctY (
            IDictionary<string, ScreenRectangle> tiles,
            IEnumerable<LayoutWindow> windows,
            string monitorId) {
            var values = new HashSet<int>();
            foreach( LayoutWindow window in windows ) {
                if( window.MonitorId == monitorId ) {
                    values.Add(GetTile(tiles, window.Id).Y);
                }
            }
            return values.Count;
        }

        private static void AssertAllInside (
            IDictionary<string, ScreenRectangle> tiles,
            IEnumerable<LayoutWindow> windows,
            LayoutMonitor monitor) {
            foreach( LayoutWindow window in windows ) {
                if( window.MonitorId == monitor.Id ) {
                    AssertInside(GetTile(tiles, window.Id), monitor.WorkingArea, window.Id);
                }
            }
        }

        private static ScreenRectangle GetTile (IDictionary<string, ScreenRectangle> tiles, string id) {
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
