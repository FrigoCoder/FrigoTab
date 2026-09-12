using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class SwitcherInputBalanceAcceptanceTests : SwitcherAcceptanceTestBase {

        [TestMethod]
        public void T20260912T090300Z_041_ConsumedAltTabKeyDownAlsoConsumesItsTabKeyUp () {
            GivenPort(3);
            SendAltTab();
            SendTabUp();

            AssertConsumed();
        }

        [TestMethod]
        public void T20260912T090300Z_042_ConsumedDigitKeyDownAlsoConsumesItsKeyUp () {
            GivenPort(3);
            SendAltTab();
            SendDigitDown(1);
            SendDigitUp(1);

            AssertConsumed();
        }

        [TestMethod]
        public void T20260912T090300Z_043_ConsumedEscapeKeyDownAlsoConsumesItsKeyUp () {
            GivenPort(3);
            SendAltTab();
            SendEscapeDown();
            SendEscapeUp();

            AssertConsumed();
        }

        [TestMethod]
        public void T20260912T090300Z_044_ConsumedAltF4KeyDownAlsoConsumesItsF4KeyUp () {
            GivenPort(3);
            SendAltTab();
            SendAltF4Down();
            SendAltF4Up();

            AssertConsumed();
        }

    }

}
