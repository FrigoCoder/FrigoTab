using System;
using System.Reflection;
using System.Runtime.InteropServices;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class KeyboardInfrastructureAcceptanceTests {

        [TestMethod]
        public void T20260912T091800Z_055_AltStateComesFromEventTransitions () {
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
        public void T20260912T091800Z_056_ShiftStateComesFromEventTransitions () {
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
        public void T20260912T091800Z_057_InjectedModifiersDoNotChangePhysicalState () {
            KeyboardModifierState state = new KeyboardModifierState();

            state.CreateInput(SwitcherKey.Alt, KeyTransition.Down, false, true);
            KeyboardInput physicalTab = state.CreateInput(SwitcherKey.Tab, KeyTransition.Down, false, false);

            Assert.IsFalse(physicalTab.Alt);
        }

        [TestMethod]
        public void T20260912T091800Z_058_SuppressionBalancesAKeyAfterSessionClose () {
            KeyboardSuppressionState state = new KeyboardSuppressionState();
            KeyboardInput altTabDown = new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, false);
            KeyboardInput tabUp = new KeyboardInput(SwitcherKey.Tab, KeyTransition.Up, true, false, false);

            Assert.IsTrue(state.ShouldConsume(altTabDown));
            state.SetSessionVisible(false);
            Assert.IsTrue(state.ShouldConsume(tabUp));
            Assert.IsFalse(state.ShouldConsume(tabUp), "Only the matching physical release is consumed.");
        }

        [TestMethod]
        public void T20260912T091800Z_059_AutoRepeatNeedsOnlyOneMatchingRelease () {
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
        public void T20260912T091800Z_060_KeyboardDeliveryIsDeferredUntilThePostedCallbackRuns () {
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
        public void T20260912T091800Z_061_FullKeyboardQueueFailsOpen () {
            DeferredKeyboardDispatcher dispatcher = new DeferredKeyboardDispatcher(
                action => true,
                input => { },
                1);
            KeyboardInput input = new KeyboardInput(SwitcherKey.Tab, KeyTransition.Down, true, false, false);

            Assert.IsTrue(dispatcher.TryDispatch(input));
            Assert.IsFalse(dispatcher.TryDispatch(input));
        }

        [TestMethod]
        public void T20260912T092900Z_068_FailedOpenReplaysACompleteGestureAfterAltWasReleased () {
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
        public void T20260912T092900Z_069_SendInputReplayUsesTheNativeInputStructureSize () {
            Type nativeInput = typeof(KeyHook).GetNestedType("NativeInput", BindingFlags.NonPublic);

            Assert.IsNotNull(nativeInput);
            Assert.AreEqual(IntPtr.Size == 8 ? 40 : 28, Marshal.SizeOf(nativeInput));
        }

        [TestMethod]
        public void T20260912T092900Z_070_ReleasingOneShiftKeyKeepsTheOtherShiftKeyDown () {
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
        public void T20260912T093400Z_074_NativeAltFlagRecoversAMissedModifierTransitionUntilAltUp () {
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
        public void T20260912T093600Z_075_FailedReverseOpenReplaysShiftWhenItWasReleasedBeforeDispatch () {
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
        public void T20260912T093600Z_076_ForwardRecoveryTemporarilyExcludesANewerShiftPress () {
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

    }

}
