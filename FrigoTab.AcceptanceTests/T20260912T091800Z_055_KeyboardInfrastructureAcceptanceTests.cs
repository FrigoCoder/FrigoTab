using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class T20260912T091800Z_055_KeyboardInfrastructureAcceptanceTests {

        [TestMethod]
        public void AltStateComesFromEventTransitions () {
            KeyboardModifierState state = new KeyboardModifierState();

            KeyboardInput altDown = state.CreateInput(SwitcherKey.Alt, KeyTransition.Down, false, false);
            KeyboardInput tabDown = state.CreateInput(SwitcherKey.Tab, KeyTransition.Down, false, false);
            KeyboardInput altUp = state.CreateInput(SwitcherKey.Alt, KeyTransition.Up, false, false);
            KeyboardInput laterTab = state.CreateInput(SwitcherKey.Tab, KeyTransition.Down, false, false);

            Assert.IsTrue(altDown.Alt);
            Assert.IsTrue(tabDown.Alt);
            Assert.IsTrue(altUp.Alt, "Alt-up must describe the state before the release is applied.");
            Assert.IsFalse(laterTab.Alt);
        }

        [TestMethod]
        public void ShiftStateComesFromEventTransitions () {
            KeyboardModifierState state = new KeyboardModifierState();

            state.CreateInput(SwitcherKey.Shift, KeyTransition.Down, false, false);
            KeyboardInput tabDown = state.CreateInput(SwitcherKey.Tab, KeyTransition.Down, true, false);
            KeyboardInput shiftUp = state.CreateInput(SwitcherKey.Shift, KeyTransition.Up, true, false);
            KeyboardInput laterTab = state.CreateInput(SwitcherKey.Tab, KeyTransition.Down, true, false);

            Assert.IsTrue(tabDown.Shift);
            Assert.IsTrue(shiftUp.Shift);
            Assert.IsFalse(laterTab.Shift);
        }

        [TestMethod]
        public void InjectedModifiersDoNotChangePhysicalState () {
            KeyboardModifierState state = new KeyboardModifierState();

            state.CreateInput(SwitcherKey.Alt, KeyTransition.Down, false, true);
            KeyboardInput physicalTab = state.CreateInput(SwitcherKey.Tab, KeyTransition.Down, false, false);

            Assert.IsFalse(physicalTab.Alt);
        }

        [TestMethod]
        public void SuppressionBalancesAKeyAfterSessionClose () {
            KeyboardSuppressionState state = new KeyboardSuppressionState();
            KeyboardInput altTabDown = new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, false);
            KeyboardInput tabUp = new KeyboardInput(SwitcherKey.Tab, KeyTransition.Up, true, false, false);

            Assert.IsTrue(state.ShouldConsume(altTabDown));
            state.SetSessionVisible(false);
            Assert.IsTrue(state.ShouldConsume(tabUp));
            Assert.IsFalse(state.ShouldConsume(tabUp), "Only the matching physical release is consumed.");
        }

        [TestMethod]
        public void AutoRepeatNeedsOnlyOneMatchingRelease () {
            KeyboardSuppressionState state = new KeyboardSuppressionState();
            state.SetSessionVisible(true);
            KeyboardInput down = new KeyboardInput(SwitcherKey.D1, KeyTransition.Down, false, false, false);
            KeyboardInput up = new KeyboardInput(SwitcherKey.D1, KeyTransition.Up, false, false, false);

            Assert.IsTrue(state.ShouldConsume(down));
            Assert.IsTrue(state.ShouldConsume(down));
            Assert.IsTrue(state.ShouldConsume(up));
            Assert.IsFalse(state.ShouldConsume(up));
        }

        [TestMethod]
        public void KeyboardDeliveryIsDeferredUntilThePostedCallbackRuns () {
            Action posted = null;
            int delivered = 0;
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => {
                    posted = action;
                    return true;
                },
                input => delivered++);

            bool accepted = dispatcher.TryDispatch(
                new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, false));

            Assert.IsTrue(accepted);
            Assert.AreEqual(0, delivered);
            Assert.AreEqual(1, dispatcher.PendingCount);
            Assert.IsNotNull(posted);
            posted();
            Assert.AreEqual(1, delivered);
            Assert.AreEqual(0, dispatcher.PendingCount);
        }

        [TestMethod]
        public void FullKeyboardQueueFailsOpen () {
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => true,
                input => { },
                1);
            KeyboardInput input = new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, false);

            Assert.IsTrue(dispatcher.TryDispatch(input));
            Assert.IsFalse(dispatcher.TryDispatch(input));
        }

        [TestMethod]
        public void FailedOpenReplaysACompleteGestureAfterAltWasReleased () {
            KeyboardInput[] replay = AltTabRecoveryPlan.Create(false);

            Assert.AreEqual(4, replay.Length);
            Assert.AreEqual(SwitcherKey.Alt, replay[0].Key);
            Assert.AreEqual(KeyTransition.Down, replay[0].Transition);
            Assert.AreEqual(SwitcherKey.Tab, replay[1].Key);
            Assert.AreEqual(KeyTransition.Down, replay[1].Transition);
            Assert.AreEqual(SwitcherKey.Tab, replay[2].Key);
            Assert.AreEqual(KeyTransition.Up, replay[2].Transition);
            Assert.AreEqual(SwitcherKey.Alt, replay[3].Key);
            Assert.AreEqual(KeyTransition.Up, replay[3].Transition);
            Assert.IsTrue(Array.TrueForAll(replay, input => input.Injected));
        }

        [TestMethod]
        public void SendInputReplayUsesTheNativeInputStructureSize () {
            Type nativeInput = typeof(KeyHook).GetNestedType("NativeInput", BindingFlags.NonPublic);

            Assert.IsNotNull(nativeInput);
            Assert.AreEqual(IntPtr.Size == 8 ? 40 : 28, Marshal.SizeOf(nativeInput));
        }

        [TestMethod]
        public void ReleasingOneShiftKeyKeepsTheOtherShiftKeyDown () {
            KeyboardModifierState state = new KeyboardModifierState();

            state.CreateInput(
                SwitcherKey.Shift,
                KeyTransition.Down,
                false,
                false,
                KeyboardModifierKey.LeftShift);
            state.CreateInput(
                SwitcherKey.Shift,
                KeyTransition.Down,
                false,
                false,
                KeyboardModifierKey.RightShift);
            state.CreateInput(
                SwitcherKey.Shift,
                KeyTransition.Up,
                false,
                false,
                KeyboardModifierKey.LeftShift);
            KeyboardInput tab = state.CreateInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                KeyboardModifierKey.None);

            Assert.IsTrue(tab.Shift);
            Assert.IsTrue(state.ShiftDown);
        }

        [TestMethod]
        public void NativeAltFlagRecoversAMissedModifierTransitionUntilAltUp () {
            KeyboardModifierState state = new KeyboardModifierState();

            KeyboardInput tab = state.CreateInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                KeyboardModifierKey.None);

            Assert.IsTrue(tab.Alt);
            Assert.IsTrue(state.AltDown);

            state.CreateInput(
                SwitcherKey.Alt,
                KeyTransition.Up,
                true,
                false,
                KeyboardModifierKey.LeftAlt);
            Assert.IsFalse(state.AltDown);
        }

        [TestMethod]
        public void FailedReverseOpenReplaysShiftWhenItWasReleasedBeforeDispatch () {
            KeyboardInput[] replay = AltTabRecoveryPlan.Create(false, true, false);

            Assert.AreEqual(6, replay.Length);
            Assert.AreEqual(SwitcherKey.Alt, replay[0].Key);
            Assert.AreEqual(KeyTransition.Down, replay[0].Transition);
            Assert.AreEqual(SwitcherKey.Shift, replay[1].Key);
            Assert.AreEqual(KeyTransition.Down, replay[1].Transition);
            Assert.AreEqual(SwitcherKey.Tab, replay[2].Key);
            Assert.AreEqual(KeyTransition.Down, replay[2].Transition);
            Assert.AreEqual(SwitcherKey.Tab, replay[3].Key);
            Assert.AreEqual(KeyTransition.Up, replay[3].Transition);
            Assert.AreEqual(SwitcherKey.Shift, replay[4].Key);
            Assert.AreEqual(KeyTransition.Up, replay[4].Transition);
            Assert.AreEqual(SwitcherKey.Alt, replay[5].Key);
            Assert.AreEqual(KeyTransition.Up, replay[5].Transition);
        }

        [TestMethod]
        public void ForwardRecoveryTemporarilyExcludesANewerShiftPress () {
            KeyboardInput[] replay = AltTabRecoveryPlan.Create(true, false, true);

            Assert.AreEqual(4, replay.Length);
            Assert.AreEqual(SwitcherKey.Shift, replay[0].Key);
            Assert.AreEqual(KeyTransition.Up, replay[0].Transition);
            Assert.AreEqual(SwitcherKey.Tab, replay[1].Key);
            Assert.AreEqual(KeyTransition.Down, replay[1].Transition);
            Assert.AreEqual(SwitcherKey.Tab, replay[2].Key);
            Assert.AreEqual(KeyTransition.Up, replay[2].Transition);
            Assert.AreEqual(SwitcherKey.Shift, replay[3].Key);
            Assert.AreEqual(KeyTransition.Down, replay[3].Transition);
        }

        [TestMethod]
        public void AbortingPendingAdmissionClearsOnlyTheFailedInitialGesture () {
            KeyboardSuppressionState state = new KeyboardSuppressionState();
            KeyboardInput tabDown = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                false);
            KeyboardInput tabUp = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Up,
                true,
                false,
                false);

            long admissionToken;
            Assert.IsTrue(state.ShouldConsume(tabDown, out admissionToken));
            Assert.AreNotEqual(0, admissionToken);
            Assert.IsTrue(state.AdmissionPending);
            Assert.IsTrue(state.AbortPendingAdmission(admissionToken));
            Assert.IsFalse(state.SessionActiveOrPending);
            Assert.IsFalse(state.AdmissionPending);
            Assert.IsFalse(state.ShouldConsume(tabUp));
        }

        [TestMethod]
        public void AbortingPendingAdmissionDoesNotDisableAnAlreadyVisibleSession () {
            KeyboardSuppressionState state = new KeyboardSuppressionState();
            KeyboardInput tabDown = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                false);
            KeyboardInput digitDown = new KeyboardInput(
                SwitcherKey.D1,
                KeyTransition.Down,
                false,
                false,
                false);

            long admissionToken;
            Assert.IsTrue(state.ShouldConsume(tabDown, out admissionToken));
            state.SetSessionVisible(true);
            Assert.IsFalse(state.AdmissionPending);
            Assert.IsFalse(state.AbortPendingAdmission(admissionToken));
            Assert.IsTrue(state.SessionActiveOrPending);
            Assert.IsTrue(state.ShouldConsume(digitDown));
        }

        [TestMethod]
        public void FailedTabRepeatCannotCancelAnAdmissionAlreadyQueuedForTheUi () {
            KeyboardSuppressionState state = new KeyboardSuppressionState();
            Queue<Action> posted = new Queue<Action>();
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => {
                    posted.Enqueue(action);
                    return true;
                },
                input => { },
                1);
            KeyboardInput tabDown = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                false);

            long originalAdmissionToken;
            Assert.IsTrue(state.ShouldConsume(tabDown, out originalAdmissionToken));
            Assert.AreNotEqual(0, originalAdmissionToken);
            Assert.IsTrue(dispatcher.TryDispatch(tabDown), "the original admission reaches the UI queue");

            long repeatAdmissionToken;
            Assert.IsTrue(state.ShouldConsume(tabDown, out repeatAdmissionToken));
            Assert.AreEqual(0, repeatAdmissionToken);
            Assert.IsFalse(dispatcher.TryDispatch(tabDown), "the repeat arrives while the bounded queue is full");
            Assert.IsFalse(state.AbortPendingAdmission(repeatAdmissionToken));
            Assert.IsTrue(state.SessionActiveOrPending);
            Assert.IsTrue(state.AdmissionPending);

            posted.Dequeue()();
            state.SetSessionVisible(true);
            Assert.IsFalse(state.AbortPendingAdmission(originalAdmissionToken));
            Assert.IsTrue(state.SessionActiveOrPending);
        }

        [TestMethod]
        public void FailedUiPostReleasesTheBoundedKeyboardDeliverySlot () {
            bool acceptPosts = false;
            Action queued = null;
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => {
                    if( !acceptPosts ) {
                        return false;
                    }
                    queued = action;
                    return true;
                },
                input => { },
                1);
            KeyboardInput input = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                false);

            Assert.IsFalse(dispatcher.TryDispatch(input));
            Assert.AreEqual(0, dispatcher.PendingCount);
            acceptPosts = true;
            Assert.IsTrue(dispatcher.TryDispatch(input));
            queued();
            Assert.AreEqual(0, dispatcher.PendingCount);
        }

        [TestMethod]
        public void SessionEndingInputUsesOneReservedSlotWhenRepeatQueueIsFull () {
            Queue<Action> posted = new Queue<Action>();
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => {
                    posted.Enqueue(action);
                    return true;
                },
                input => { },
                1);
            KeyboardInput repeatedTab = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                false);
            KeyboardInput altUp = new KeyboardInput(
                SwitcherKey.Alt,
                KeyTransition.Up,
                true,
                false,
                false);

            Assert.IsTrue(dispatcher.TryDispatch(repeatedTab));
            Assert.IsFalse(dispatcher.TryDispatch(repeatedTab));
            Assert.IsTrue(dispatcher.TryDispatchCritical(altUp));
            Assert.IsFalse(dispatcher.TryDispatchCritical(altUp));
            Assert.AreEqual(2, dispatcher.PendingCount);

            while( posted.Count > 0 ) {
                posted.Dequeue()();
            }
            Assert.AreEqual(0, dispatcher.PendingCount);
        }

        [TestMethod]
        public void InputResetInvalidatesEventsQueuedBeforeTheInterruption () {
            Queue<Action> posted = new Queue<Action>();
            List<KeyboardInput> delivered = new List<KeyboardInput>();
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => {
                    posted.Enqueue(action);
                    return true;
                },
                delivered.Add,
                2);
            KeyboardInput staleTab = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                false,
                false);
            KeyboardInput currentTab = new KeyboardInput(
                SwitcherKey.Tab,
                KeyTransition.Down,
                true,
                true,
                false);

            Assert.IsTrue(dispatcher.TryDispatch(staleTab));
            dispatcher.InvalidatePending();
            posted.Dequeue()();
            Assert.AreEqual(0, delivered.Count, "an event from the interrupted session must not reopen it");
            Assert.AreEqual(0, dispatcher.PendingCount);

            Assert.IsTrue(dispatcher.TryDispatch(currentTab));
            posted.Dequeue()();
            CollectionAssert.AreEqual(new[] { currentTab }, delivered);
        }

        [TestMethod]
        public void GlobalHookOwnsADedicatedMessageLoopThread () {
            BindingFlags flags = BindingFlags.Instance | BindingFlags.NonPublic;
            Assert.IsNotNull(typeof(KeyHook).GetField("hookThread", flags));
            Assert.IsNotNull(typeof(KeyHook).GetMethod("HookThreadMain", flags));
        }

    }

}
