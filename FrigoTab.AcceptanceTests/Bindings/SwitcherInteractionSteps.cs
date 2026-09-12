using System;
using System.Collections.Generic;
using FrigoTab.AcceptanceTests.Support;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Reqnroll;

namespace FrigoTab.AcceptanceTests.Bindings {

    [Binding]
    public sealed class SwitcherInteractionSteps {

        private FakeSwitcherSessionPort port;
        private SwitcherApplication application;
        private KeyHandling lastHandling;

        [Given(@"a switcher port with (\d+) candidates")]
        public void GivenASwitcherPortWithCandidates (int count) {
            port = new FakeSwitcherSessionPort(count);
            application = new SwitcherApplication(port);
        }

        [Given(@"an empty switcher port")]
        public void GivenAnEmptySwitcherPort () {
            GivenASwitcherPortWithCandidates(0);
        }

        [Given(@"a switcher port that throws while opening")]
        public void GivenASwitcherPortThatThrowsWhileOpening () {
            GivenASwitcherPortWithCandidates(3);
            port.ThrowOnOpen = true;
        }

        [Given(@"activation will fail")]
        public void GivenActivationWillFail () {
            RequireHarness();
            port.ActivationResult = false;
        }

        [Given(@"relayout will fail")]
        public void GivenRelayoutWillFail () {
            RequireHarness();
            port.ThrowOnRelayout = true;
        }

        [Given(@"selection will fail")]
        public void GivenSelectionWillFail () {
            RequireHarness();
            port.ThrowOnSelect = true;
        }

        [Given(@"hit testing will fail")]
        public void GivenHitTestingWillFail () {
            RequireHarness();
            port.ThrowOnHitTest = true;
        }

        [Given(@"clearing selection will fail")]
        public void GivenClearingSelectionWillFail () {
            RequireHarness();
            port.ThrowOnClearSelection = true;
        }

        [Given(@"activation will throw")]
        public void GivenActivationWillThrow () {
            RequireHarness();
            port.ThrowOnActivation = true;
        }

        [Given(@"closing will fail")]
        public void GivenClosingWillFail () {
            RequireHarness();
            port.ThrowOnClose = true;
        }

        [Given(@"the pointer hit test returns candidate (\d+)")]
        public void GivenThePointerHitTestReturnsCandidate (int index) {
            RequireHarness();
            port.HitTestResult = index;
        }

        [Given(@"the pointer hit test returns no candidate")]
        public void GivenThePointerHitTestReturnsNoCandidate () {
            RequireHarness();
            port.HitTestResult = null;
        }

        [Given(@"the pointer hit test returns invalid candidate (-?\d+)")]
        public void GivenThePointerHitTestReturnsInvalidCandidate (int index) {
            RequireHarness();
            Assert.IsTrue(
                index < 0 || index >= port.CandidateCount,
                "The scenario must supply an index outside the candidate list.");
            port.HitTestResult = index;
        }

        [When(@"^I send Alt\+Tab$")]
        public void WhenISendAltTab () {
            Send(new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, false));
        }

        [When(@"^I send Alt\+Tab again$")]
        public void WhenISendAltTabAgain () {
            WhenISendAltTab();
        }

        [When(@"I send Alt\+Tab (\d+) more times")]
        public void WhenISendAltTabMoreTimes (int count) {
            for( int index = 0; index < count; index++ ) {
                WhenISendAltTab();
            }
        }

        [When(@"^I send Shift\+Alt\+Tab$")]
        public void WhenISendShiftAltTab () {
            Send(new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, true, false));
        }

        [When(@"^I send an injected Alt\+Tab$")]
        public void WhenISendAnInjectedAltTab () {
            Send(new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, true));
        }

        [When(@"I send an unrelated key-down")]
        public void WhenISendAnUnrelatedKeyDown () {
            Send(new KeyboardInput(SwitcherKey.Unknown, KeyTransition.Down, false, false, false));
        }

        [When(@"I send a Tab key-up")]
        public void WhenISendATabKeyUp () {
            Send(new KeyboardInput(SwitcherKey.Tab, KeyTransition.Up, true, false, false));
        }

        [When(@"I send the Alt key-up")]
        public void WhenISendTheAltKeyUp () {
            Send(new KeyboardInput(SwitcherKey.Alt, KeyTransition.Up, false, false, false));
        }

        [When(@"I send Escape key-down")]
        public void WhenISendEscapeKeyDown () {
            Send(new KeyboardInput(SwitcherKey.Escape, KeyTransition.Down, false, false, false));
        }

        [When(@"I send Escape key-up")]
        public void WhenISendEscapeKeyUp () {
            Send(new KeyboardInput(SwitcherKey.Escape, KeyTransition.Up, false, false, false));
        }

        [When(@"^I send Alt\+F4 key-down$")]
        public void WhenISendAltF4KeyDown () {
            Send(new KeyboardInput(SwitcherKey.F4, KeyTransition.Down, true, false, false));
        }

        [When(@"^I send Alt\+F4 key-up$")]
        public void WhenISendAltF4KeyUp () {
            Send(new KeyboardInput(SwitcherKey.F4, KeyTransition.Up, true, false, false));
        }

        [When(@"^I send D([1-9]) key-down$")]
        public void WhenISendDigitKeyDown (int digit) {
            Send(new KeyboardInput(DigitKey(digit, false), KeyTransition.Down, false, false, false));
        }

        [When(@"^I send D([1-9]) key-up$")]
        public void WhenISendDigitKeyUp (int digit) {
            Send(new KeyboardInput(DigitKey(digit, false), KeyTransition.Up, false, false, false));
        }

        [When(@"^I send NumPad([1-9]) key-down$")]
        public void WhenISendNumPadKeyDown (int digit) {
            Send(new KeyboardInput(DigitKey(digit, true), KeyTransition.Down, false, false, false));
        }

        [When(@"I move the pointer")]
        public void WhenIMoveThePointer () {
            RequireHarness();
            application.HandleMouseMove(new ScreenPoint(100, 100));
        }

        [When(@"I click the pointer")]
        public void WhenIClickThePointer () {
            RequireHarness();
            application.HandleMouseClick(new ScreenPoint(100, 100));
        }

        [When(@"the active session is interrupted")]
        public void WhenTheActiveSessionIsInterrupted () {
            RequireHarness();
            application.Interrupt();
        }

        [When(@"the display topology changes")]
        public void WhenTheDisplayTopologyChanges () {
            RequireHarness();
            application.Relayout();
        }

        [Then(@"the session is visible with candidate (\d+) selected")]
        public void ThenTheSessionIsVisibleWithCandidateSelected (int index) {
            RequireHarness();
            Assert.AreEqual(SwitcherState.Visible, application.State);
            Assert.IsTrue(application.SelectedIndex.HasValue, "A visible session should have a selected candidate.");
            Assert.AreEqual(index, application.SelectedIndex.Value);
        }

        [Then(@"the session is visible with no candidate selected")]
        public void ThenTheSessionIsVisibleWithNoCandidateSelected () {
            RequireHarness();
            Assert.AreEqual(SwitcherState.Visible, application.State);
            Assert.IsFalse(application.SelectedIndex.HasValue, "No candidate should be selected outside every tile.");
        }

        [Then(@"the session is idle")]
        public void ThenTheSessionIsIdle () {
            RequireHarness();
            Assert.AreEqual(SwitcherState.Idle, application.State);
        }

        [Then(@"the keyboard event is consumed")]
        public void ThenTheKeyboardEventIsConsumed () {
            Assert.AreEqual(KeyHandling.Consume, lastHandling);
        }

        [Then(@"the keyboard event is passed through")]
        public void ThenTheKeyboardEventIsPassedThrough () {
            Assert.AreEqual(KeyHandling.PassThrough, lastHandling);
        }

        [Then(@"the port opened (\d+) times?")]
        public void ThenThePortOpenedTimes (int count) {
            RequireHarness();
            Assert.AreEqual(count, port.OpenCalls);
        }

        [Then(@"the port was closed (\d+) times?")]
        public void ThenThePortWasClosedTimes (int count) {
            RequireHarness();
            Assert.AreEqual(count, port.CloseCalls);
        }

        [Then(@"activation was attempted (\d+) times?")]
        public void ThenActivationWasAttemptedTimes (int count) {
            RequireHarness();
            Assert.AreEqual(count, port.ActivateCalls);
        }

        [Then(@"activation was not attempted")]
        public void ThenActivationWasNotAttempted () {
            RequireHarness();
            Assert.AreEqual(0, port.ActivateCalls);
        }

        [Then(@"candidate (\d+) was activated")]
        public void ThenCandidateWasActivated (int index) {
            RequireHarness();
            Assert.AreEqual(1, port.ActivateCalls);
            int? activatedIndex = port.ActivatedSelections[0];
            Assert.IsTrue(activatedIndex.HasValue, "Activation should have a selected candidate.");
            Assert.AreEqual(index, activatedIndex.Value);
        }

        [Then(@"the selected candidates were (.*)")]
        public void ThenTheSelectedCandidatesWere (string values) {
            RequireHarness();
            var expected = new List<int>();
            foreach( string value in values.Split(',') ) {
                expected.Add(Int32.Parse(value.Trim()));
            }

            CollectionAssert.AreEqual(expected, new List<int>(port.SelectedIndices));
        }

        [Then(@"the port was relaid out (\d+) times?")]
        public void ThenThePortWasRelaidOutTimes (int count) {
            RequireHarness();
            Assert.AreEqual(count, port.RelayoutCalls);
        }

        private void Send (KeyboardInput input) {
            RequireHarness();
            lastHandling = application.HandleKeyboard(input);
        }

        private void RequireHarness () {
            if( port == null || application == null ) {
                throw new InvalidOperationException("The switcher harness has not been configured for this scenario.");
            }
        }

        private static SwitcherKey DigitKey (int digit, bool numPad) {
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
