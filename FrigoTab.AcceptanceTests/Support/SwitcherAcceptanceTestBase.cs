using System;
using System.Collections.Generic;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests.Support {

    /// <summary>
    /// Small in-memory harness for the switcher acceptance requirements.
    /// The tests call the same policy entry points as the production session,
    /// while this port keeps HWNDs, hooks, and DWM out of deterministic tests.
    /// </summary>
    public abstract class SwitcherAcceptanceTestBase {

        protected FakeSwitcherSessionPort Port { get; private set; }
        protected SwitcherApplication Application { get; private set; }
        protected KeyHandling LastHandling { get; private set; }

        protected void GivenPort (int candidateCount) {
            Port = new FakeSwitcherSessionPort(candidateCount);
            Application = new SwitcherApplication(Port);
        }

        protected void RequireHarness () {
            Assert.IsNotNull(Port, "The switcher test port was not configured.");
            Assert.IsNotNull(Application, "The switcher application was not configured.");
        }

        protected void Send (KeyboardInput input) {
            RequireHarness();
            LastHandling = Application.HandleKeyboard(input);
        }

        protected void SendAltTab (bool shift = false, bool injected = false) {
            Send(new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, shift, injected));
        }

        protected void SendTabUp () {
            Send(new KeyboardInput(SwitcherKey.Tab, KeyTransition.Up, true, false, false));
        }

        protected void SendAltUp () {
            Send(new KeyboardInput(SwitcherKey.Alt, KeyTransition.Up, false, false, false));
        }

        protected void SendEscapeDown () {
            Send(new KeyboardInput(SwitcherKey.Escape, KeyTransition.Down, false, false, false));
        }

        protected void SendEscapeUp () {
            Send(new KeyboardInput(SwitcherKey.Escape, KeyTransition.Up, false, false, false));
        }

        protected void SendAltF4Down () {
            Send(new KeyboardInput(SwitcherKey.F4, KeyTransition.Down, true, false, false));
        }

        protected void SendAltF4Up () {
            Send(new KeyboardInput(SwitcherKey.F4, KeyTransition.Up, true, false, false));
        }

        protected void SendDigitDown (int digit) {
            Send(new KeyboardInput(DigitKey(digit, false), KeyTransition.Down, false, false, false));
        }

        protected void SendDigitUp (int digit) {
            Send(new KeyboardInput(DigitKey(digit, false), KeyTransition.Up, false, false, false));
        }

        protected void SendNumPadDown (int digit) {
            Send(new KeyboardInput(DigitKey(digit, true), KeyTransition.Down, false, false, false));
        }

        protected void MovePointer () {
            RequireHarness();
            Application.HandleMouseMove(new ScreenPoint(100, 100));
        }

        protected void ClickPointer () {
            RequireHarness();
            Application.HandleMouseClick(new ScreenPoint(100, 100));
        }

        protected void Interrupt () {
            RequireHarness();
            Application.Interrupt();
        }

        protected void Relayout () {
            RequireHarness();
            Application.Relayout();
        }

        protected void AssertVisible (int selectedIndex) {
            RequireHarness();
            Assert.AreEqual(SwitcherState.Visible, Application.State);
            Assert.IsTrue(Application.SelectedIndex.HasValue, "A visible session should have a selected candidate.");
            Assert.AreEqual(selectedIndex, Application.SelectedIndex.Value);
        }

        protected void AssertVisibleWithoutSelection () {
            RequireHarness();
            Assert.AreEqual(SwitcherState.Visible, Application.State);
            Assert.IsFalse(Application.SelectedIndex.HasValue, "No candidate should be selected outside every tile.");
        }

        protected void AssertIdle () {
            RequireHarness();
            Assert.AreEqual(SwitcherState.Idle, Application.State);
        }

        protected void AssertConsumed () {
            Assert.AreEqual(KeyHandling.Consume, LastHandling);
        }

        protected void AssertPassedThrough () {
            Assert.AreEqual(KeyHandling.PassThrough, LastHandling);
        }

        protected void AssertOpened (int count) {
            RequireHarness();
            Assert.AreEqual(count, Port.OpenCalls);
        }

        protected void AssertClosed (int count) {
            RequireHarness();
            Assert.AreEqual(count, Port.CloseCalls);
        }

        protected void AssertActivationAttempts (int count) {
            RequireHarness();
            Assert.AreEqual(count, Port.ActivateCalls);
        }

        protected void AssertNoActivation () {
            AssertActivationAttempts(0);
        }

        protected void AssertActivated (int index) {
            RequireHarness();
            Assert.AreEqual(1, Port.ActivateCalls);
            int? activatedIndex = Port.ActivatedSelections[0];
            Assert.IsTrue(activatedIndex.HasValue, "Activation should have a selected candidate.");
            Assert.AreEqual(index, activatedIndex.Value);
        }

        protected void AssertSelectedCandidates (params int[] expected) {
            RequireHarness();
            CollectionAssert.AreEqual(new List<int>(expected), new List<int>(Port.SelectedIndices));
        }

        protected void AssertRelayouts (int count) {
            RequireHarness();
            Assert.AreEqual(count, Port.RelayoutCalls);
        }

        protected static SwitcherKey DigitKey (int digit, bool numPad) {
            if( digit < 1 || digit > 9 ) {
                throw new ArgumentOutOfRangeException(nameof(digit));
            }

            if( numPad ) {
                return (SwitcherKey) ((int) SwitcherKey.NumPad1 + digit - 1);
            }
            return (SwitcherKey) ((int) SwitcherKey.D1 + digit - 1);
        }

    }

}
