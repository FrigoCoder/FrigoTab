using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Linq;
using System.Threading;
using System.Windows.Forms;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [STATestClass]
    [DoNotParallelize]
    [TestCategory("Acceptance")]
    public class T20260914T211700Z_001_RealSwitcherAcceptanceTests {

        [TestMethod]
        public void AltTabOpensTheRealSwitcherAndSelectsARealPreview () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();

                Assert.IsTrue(desktop.Form.Visible);
                Assert.AreEqual(SwitcherState.Visible, desktop.Switcher.State);
                Assert.AreEqual(desktop.Switcher.CandidateCount, desktop.Tiles.Count);
                Assert.AreEqual(0, desktop.Switcher.SelectedIndex);
                Assert.AreEqual(1, desktop.Tiles.Count(IsSelected));
            }
        }

        [TestMethod]
        public void HoldingAltCyclesForwardAndWrapsThroughRealPreviews () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                int count = desktop.Switcher.CandidateCount;
                Assert.IsTrue(count >= 3);

                for( int index = 1; index < count; index++ ) {
                    Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true));
                    Assert.AreEqual(index, desktop.Switcher.SelectedIndex);
                }
                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true));
                Assert.AreEqual(0, desktop.Switcher.SelectedIndex);
            }
        }

        [TestMethod]
        public void ShiftAltTabCyclesBackwardAndWrapsThroughRealPreviews () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                int last = desktop.Switcher.CandidateCount - 1;

                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true, shift: true));
                Assert.AreEqual(last, desktop.Switcher.SelectedIndex);

                for( int index = last - 1; index >= 0; index-- ) {
                    desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true, shift: true);
                    Assert.AreEqual(index, desktop.Switcher.SelectedIndex);
                }
            }
        }

        [TestMethod]
        public void ReleasingAltLeavesTheRealSwitcherOpenForDeliberateSelection () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                ApplicationWindow target = desktop.FixtureTiles.First(tile => !IsSelected(tile));
                desktop.Switcher.HandleMouseMove(desktop.Center(target));
                Assert.AreSame(target, desktop.SelectedTile);

                Assert.AreEqual(KeyHandling.PassThrough, desktop.Key(SwitcherKey.Alt, KeyTransition.Up));

                Assert.AreEqual(SwitcherState.Visible, desktop.Switcher.State);
                Assert.IsTrue(desktop.Form.Visible);
                Assert.AreSame(target, desktop.SelectedTile);

                desktop.Switcher.HandleMouseClick(desktop.Center(target));
                desktop.Pump();
                desktop.AssertClosed();
                desktop.AssertForeground(target.Application);
            }
        }

        [TestMethod]
        public void EscapeCancelsTheRealSession () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();

                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.Escape, KeyTransition.Down));

                desktop.AssertClosed();
            }
        }

        [TestMethod]
        public void AltF4CancelsTheRealSession () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();

                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.F4, KeyTransition.Down, alt: true));

                desktop.AssertClosed();
            }
        }

        [TestMethod]
        public void NumberOneActivatesTheFirstRealPreview () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                WindowHandle expected = desktop.SelectedTile.Application;

                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.D1, KeyTransition.Down));

                desktop.AssertClosed();
                desktop.AssertForeground(expected);
            }
        }

        [TestMethod]
        public void NumberPadTwoActivatesTheSecondRealPreview () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true);
                WindowHandle expected = desktop.SelectedTile.Application;
                desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true, shift: true);

                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.NumPad2, KeyTransition.Down));

                desktop.AssertClosed();
                desktop.AssertForeground(expected);
            }
        }

        [TestMethod]
        public void PointerHoverAndClickSelectAndActivateTheExactRealPreview () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                ApplicationWindow target = desktop.FixtureTiles.First(tile => !IsSelected(tile));
                ScreenPoint point = desktop.Center(target);

                desktop.Switcher.HandleMouseMove(point);
                Assert.AreSame(target, desktop.SelectedTile);
                desktop.Switcher.HandleMouseClick(point);
                desktop.Pump();

                desktop.AssertClosed();
                desktop.AssertForeground(target.Application);
            }
        }

        [TestMethod]
        public void MovingOutsideTheTilesClearsSelectionAndAltTabRestoresIt () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();

                desktop.Switcher.HandleMouseMove(desktop.PointOutsideTiles());
                Assert.IsNull(desktop.Switcher.SelectedIndex);
                Assert.AreEqual(0, desktop.Tiles.Count(IsSelected));

                desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true);
                Assert.AreEqual(0, desktop.Switcher.SelectedIndex);
                Assert.AreEqual(1, desktop.Tiles.Count(IsSelected));
            }
        }

        [TestMethod]
        public void AClosedTargetLeavesTheRealSessionAvailableForCancellation () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                ApplicationWindow target = desktop.FixtureTiles.First();
                desktop.Switcher.HandleMouseMove(desktop.Center(target));
                desktop.CloseFixture(target.Application);

                desktop.Key(SwitcherKey.Alt, KeyTransition.Up);

                Assert.AreEqual(SwitcherState.Visible, desktop.Switcher.State);
                Assert.IsTrue(desktop.Form.Visible);
                desktop.Key(SwitcherKey.Escape, KeyTransition.Down);
                desktop.AssertClosed();
            }
        }

        [TestMethod]
        public void ClosingAndReopeningRecreatesRealNativePreviews () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                int count = desktop.Tiles.Count;
                desktop.Key(SwitcherKey.Escape, KeyTransition.Down);
                desktop.AssertClosed();

                desktop.Open();

                Assert.AreEqual(count, desktop.Tiles.Count);
                Assert.AreEqual(1, desktop.Tiles.Count(IsSelected));
            }
        }

        [TestMethod]
        public void InjectedAltTabPassesThroughWithoutOpeningRealWindows () {
            using( LiveSession desktop = new LiveSession() ) {
                Assert.AreEqual(
                    KeyHandling.PassThrough,
                    desktop.Key(SwitcherKey.Tab, KeyTransition.Down, alt: true, injected: true));
                Assert.AreEqual(SwitcherState.Idle, desktop.Switcher.State);
                Assert.IsFalse(desktop.Form.Visible);
                Assert.AreEqual(0, desktop.Tiles.Count);
            }
        }

        [TestMethod]
        public void ConsumedKeyUpsRemainBalancedAfterTheRealSessionCloses () {
            using( LiveSession desktop = new LiveSession() ) {
                desktop.Open();
                desktop.Key(SwitcherKey.Escape, KeyTransition.Down);
                desktop.AssertClosed();

                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.Tab, KeyTransition.Up));
                Assert.AreEqual(KeyHandling.Consume, desktop.Key(SwitcherKey.Escape, KeyTransition.Up));
                Assert.AreEqual(KeyHandling.PassThrough, desktop.Key(SwitcherKey.Tab, KeyTransition.Up));
            }
        }

        [TestMethod]
        public void RepeatedOpenAndCloseLeavesNoPreviewFormsBehind () {
            using( LiveSession desktop = new LiveSession() ) {
                for( int repetition = 0; repetition < 5; repetition++ ) {
                    desktop.Open();
                    Assert.IsTrue(desktop.Tiles.Count > 0);
                    desktop.Key(SwitcherKey.Escape, KeyTransition.Down);
                    desktop.AssertClosed();
                    Assert.AreEqual(0, desktop.Tiles.Count);
                }
            }
        }

        private static bool IsSelected (ApplicationWindow tile) {
            Property<bool> selected = tile.Selected;
            return selected.Value;
        }

        private sealed class LiveSession : IDisposable {

            private readonly List<Form> fixtures = new List<Form>();
            private readonly Dictionary<WindowHandle, Form> fixturesByHandle = new Dictionary<WindowHandle, Form>();
            private readonly WindowHandle previousForeground;

            public LiveSession () {
                previousForeground = WindowHandle.GetForegroundWindow();
                Form = new SessionForm();
                _ = Form.Handle;

                Color[] colors = {Color.Magenta, Color.Lime, Color.Orange};
                for( int index = 0; index < colors.Length; index++ ) {
                    Form fixture = new Form {
                        Text = "FrigoTab real acceptance window " + (index + 1),
                        BackColor = colors[index],
                        Bounds = new Rectangle(80 + index * 90, 80 + index * 70, 640, 480),
                        StartPosition = FormStartPosition.Manual,
                        ShowInTaskbar = true
                    };
                    fixture.Show();
                    fixture.Refresh();
                    fixtures.Add(fixture);
                    fixturesByHandle.Add(new WindowHandle(fixture.Handle), fixture);
                }
                Pump();
                Switcher = new SwitcherApplication((ISwitcherSessionPort) Form);
            }

            public SessionForm Form { get; }
            public SwitcherApplication Switcher { get; }

            public List<ApplicationWindow> Tiles => Application.OpenForms
                .Cast<Form>()
                .OfType<ApplicationWindow>()
                .Where(tile => ReferenceEquals(tile.Owner, Form))
                .ToList();

            public List<ApplicationWindow> FixtureTiles => Tiles
                .Where(tile => fixturesByHandle.ContainsKey(tile.Application))
                .ToList();

            public ApplicationWindow SelectedTile => Tiles.Single(IsSelected);

            public void Open () {
                Assert.AreEqual(KeyHandling.Consume, Key(SwitcherKey.Tab, KeyTransition.Down, alt: true));
                WaitUntil(() => Form.Visible && Tiles.Count > 0);
                Assert.AreEqual(SwitcherState.Visible, Switcher.State);
            }

            public KeyHandling Key (
                SwitcherKey key,
                KeyTransition transition,
                bool alt = false,
                bool shift = false,
                bool injected = false) {
                KeyHandling result = Switcher.HandleKeyboard(new KeyboardInput(key, transition, alt, shift, injected));
                Pump();
                return result;
            }

            public ScreenPoint Center (ApplicationWindow tile) => new ScreenPoint(
                tile.Bounds.Left + tile.Bounds.Width / 2,
                tile.Bounds.Top + tile.Bounds.Height / 2);

            public ScreenPoint PointOutsideTiles () {
                Rectangle desktop = SystemInformation.VirtualScreen;
                for( int y = desktop.Top; y < desktop.Bottom; y += 8 ) {
                    for( int x = desktop.Left; x < desktop.Right; x += 8 ) {
                        Point point = new Point(x, y);
                        if( Tiles.All(tile => !tile.Bounds.Contains(point)) ) {
                            return new ScreenPoint(x, y);
                        }
                    }
                }
                Assert.Fail("No point outside the preview windows was available.");
                return default(ScreenPoint);
            }

            public void CloseFixture (WindowHandle handle) {
                Form fixture = fixturesByHandle[handle];
                fixturesByHandle.Remove(handle);
                fixture.Close();
                fixture.Dispose();
                fixtures.Remove(fixture);
                Pump();
            }

            public void AssertClosed () {
                WaitUntil(() => Switcher.State == SwitcherState.Idle && !Form.Visible);
            }

            public void AssertForeground (WindowHandle expected) {
                WaitUntil(() => WindowHandle.GetForegroundWindow() == expected);
            }

            public void Pump () {
                Application.DoEvents();
                Thread.Sleep(20);
                Application.DoEvents();
            }

            public void Dispose () {
                Switcher.Close();
                Form.Dispose();
                foreach( Form fixture in fixtures ) {
                    fixture.Close();
                    fixture.Dispose();
                }
                fixtures.Clear();
                fixturesByHandle.Clear();
                if( previousForeground != WindowHandle.Null ) {
                    previousForeground.SetForeground();
                }
                Pump();
            }

            private void WaitUntil (Func<bool> condition) {
                Stopwatch timeout = Stopwatch.StartNew();
                while( timeout.Elapsed < TimeSpan.FromSeconds(5) ) {
                    if( condition() ) {
                        return;
                    }
                    Pump();
                }
                Assert.Fail("Timed out waiting for the real desktop state.");
            }

        }

    }

}
