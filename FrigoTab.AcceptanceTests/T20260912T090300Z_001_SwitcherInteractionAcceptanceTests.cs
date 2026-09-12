using FrigoTab.Core;
using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("CurrentFeature")]
    [TestCategory("Regression")]
    public sealed class T20260912T090300Z_001_SwitcherInteractionAcceptanceTests : SwitcherAcceptanceTestBase {

        [TestMethod]
        public void FirstAltTabOpensAndSelectsTheFirstCandidate () {
            GivenPort(3);
            SendAltTab();

            AssertVisible(0);
            AssertConsumed();
            AssertOpened(1);
        }

        [TestMethod]
        public void EmptyCandidateListFailsOpenAndCleansUp () {
            GivenPort(0);
            SendAltTab();

            AssertPassedThrough();
            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void ExceptionWhileOpeningFailsOpenAndCleansUp () {
            GivenPort(3);
            Port.ThrowOnOpen = true;
            SendAltTab();

            AssertPassedThrough();
            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void ExceptionWhileSelectingInitialCandidateFailsOpenAndCleansUp () {
            GivenPort(3);
            Port.ThrowOnSelect = true;
            SendAltTab();

            AssertPassedThrough();
            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void UnrelatedKeyIsPassedThroughWhileIdle () {
            GivenPort(3);
            Send(new KeyboardInput(SwitcherKey.Unknown, KeyTransition.Down, false, false, false));

            AssertPassedThrough();
            AssertOpened(0);
        }

        [TestMethod]
        public void InjectedAltTabIsPassedThrough () {
            GivenPort(3);
            SendAltTab(injected: true);

            AssertPassedThrough();
            AssertOpened(0);
        }

        [TestMethod]
        public void KeyUpEventIsPassedThroughWhileIdle () {
            GivenPort(3);
            SendTabUp();

            AssertPassedThrough();
            AssertOpened(0);
        }

        [TestMethod]
        public void RepeatedAltTabAdvancesAndWrapsForward () {
            GivenPort(3);
            SendAltTab();
            SendAltTab();
            SendAltTab();
            SendAltTab();

            AssertVisible(0);
            AssertSelectedCandidates(0, 1, 2, 0);
        }

        [TestMethod]
        public void ShiftAltTabMovesBackwardsAndWraps () {
            GivenPort(3);
            SendAltTab();
            SendAltTab(shift: true);

            AssertVisible(2);
        }

        [TestMethod]
        public void ReleasingAltActivatesTheSelectedCandidateAndCloses () {
            GivenPort(3);
            SendAltTab();
            SendAltUp();

            AssertActivated(0);
            AssertIdle();
            AssertClosed(1);
            AssertPassedThrough();
        }

        [TestMethod]
        public void EscapeCancelsWithoutActivation () {
            GivenPort(3);
            SendAltTab();
            SendEscapeDown();

            AssertIdle();
            AssertNoActivation();
            AssertClosed(1);
        }

        [TestMethod]
        public void AltF4CancelsWithoutActivation () {
            GivenPort(3);
            SendAltTab();
            SendAltF4Down();

            AssertIdle();
            AssertNoActivation();
            AssertClosed(1);
        }

        [TestMethod]
        public void D1SelectsAndActivatesTheFirstCandidate () {
            GivenPort(3);
            SendAltTab();
            SendDigitDown(1);

            AssertActivated(0);
            AssertIdle();
        }

        [TestMethod]
        public void NumPad2SelectsAndActivatesTheSecondCandidate () {
            GivenPort(3);
            SendAltTab();
            SendNumPadDown(2);

            AssertActivated(1);
            AssertIdle();
        }

        [TestMethod]
        public void InvalidDigitPreservesTheCurrentSelection () {
            GivenPort(3);
            SendAltTab();
            SendDigitDown(9);

            AssertVisible(0);
            AssertNoActivation();
        }

        [TestMethod]
        public void PointerHoverSelectsTheCandidateUnderThePointer () {
            GivenPort(3);
            Port.HitTestResult = 2;
            SendAltTab();
            MovePointer();

            AssertVisible(2);
        }

        [TestMethod]
        public void PointerClickActivatesTheCandidateUnderThePointer () {
            GivenPort(3);
            Port.HitTestResult = 1;
            SendAltTab();
            ClickPointer();

            AssertActivated(1);
            AssertIdle();
        }

        [TestMethod]
        public void PointerOutsideEveryTileClearsSelectionWithoutActivation () {
            GivenPort(3);
            Port.HitTestResult = null;
            SendAltTab();
            MovePointer();

            AssertVisibleWithoutSelection();
            AssertNoActivation();
        }

        [TestMethod]
        public void AltTabRestoresKeyboardSelectionAfterPointerClearsIt () {
            GivenPort(3);
            Port.HitTestResult = null;
            SendAltTab();
            MovePointer();
            SendAltTab();

            AssertVisible(0);
            AssertSelectedCandidates(0, 0);
            AssertConsumed();
        }

        [TestMethod]
        public void ShiftAltTabRestoresTheLastSelectionAfterPointerClearsIt () {
            GivenPort(3);
            Port.HitTestResult = null;
            SendAltTab();
            MovePointer();
            SendAltTab(shift: true);

            AssertVisible(2);
        }

        [TestMethod]
        public void InvalidPointerCandidateClosesTheInconsistentSessionSafely () {
            GivenPort(3);
            Port.HitTestResult = 99;
            SendAltTab();
            ClickPointer();

            AssertIdle();
            AssertNoActivation();
            AssertClosed(1);
        }

        [TestMethod]
        public void HitTestFailureClosesTheSessionSafely () {
            GivenPort(3);
            Port.ThrowOnHitTest = true;
            SendAltTab();
            MovePointer();

            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void SelectionClearFailureClosesTheSessionSafely () {
            GivenPort(3);
            Port.HitTestResult = null;
            Port.ThrowOnClearSelection = true;
            SendAltTab();
            MovePointer();

            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void ActivationFailureLeavesTheSessionVisibleForRetryOrCancel () {
            GivenPort(3);
            Port.ActivationResult = false;
            SendAltTab();
            SendAltUp();

            AssertVisible(0);
            AssertActivationAttempts(1);
            AssertClosed(0);
        }

        [TestMethod]
        public void ActivationExceptionLeavesTheSessionVisibleForRetryOrCancel () {
            GivenPort(3);
            Port.ThrowOnActivation = true;
            SendAltTab();
            SendAltUp();

            AssertVisible(0);
            AssertActivationAttempts(1);
            AssertClosed(0);
        }

        [TestMethod]
        public void InterruptionReturnsTheApplicationToIdle () {
            GivenPort(3);
            SendAltTab();
            Interrupt();

            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void CleanupExceptionCannotLeaveTheApplicationActive () {
            GivenPort(3);
            Port.ThrowOnClose = true;
            SendAltTab();
            SendEscapeDown();

            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void DisplayChangeRelayoutsAVisibleSession () {
            GivenPort(3);
            SendAltTab();
            Relayout();

            AssertVisible(0);
            AssertRelayouts(1);
        }

        [TestMethod]
        public void RelayoutFailureClosesTheVisibleSessionSafely () {
            GivenPort(3);
            Port.ThrowOnRelayout = true;
            SendAltTab();
            Relayout();

            AssertIdle();
            AssertClosed(1);
        }

        [TestMethod]
        public void RelayoutFailureClearsConsumedKeysFromTheInterruptedSession () {
            GivenPort(3);
            Port.ThrowOnRelayout = true;
            SendAltTab();
            Relayout();
            SendTabUp();

            AssertIdle();
            AssertPassedThrough();
        }

        [TestMethod]
        public void ClosedSessionCanBeOpenedAgain () {
            GivenPort(3);
            SendAltTab();
            SendAltUp();
            SendAltTab();

            AssertVisible(0);
            AssertOpened(2);
        }

    }

}
