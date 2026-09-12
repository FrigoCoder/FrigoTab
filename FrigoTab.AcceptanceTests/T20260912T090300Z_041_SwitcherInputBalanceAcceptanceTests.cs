using FrigoTab.AcceptanceTests.Support;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class T20260912T090300Z_041_SwitcherInputBalanceAcceptanceTests : SwitcherAcceptanceTestBase {

        [TestMethod]
        public void ConsumedAltTabKeyDownAlsoConsumesItsTabKeyUp () {
            GivenPort(3);
            SendAltTab();
            SendTabUp();

            AssertConsumed();
        }

        [TestMethod]
        public void ConsumedDigitKeyDownAlsoConsumesItsKeyUp () {
            GivenPort(3);
            SendAltTab();
            SendDigitDown(1);
            SendDigitUp(1);

            AssertConsumed();
        }

        [TestMethod]
        public void ConsumedEscapeKeyDownAlsoConsumesItsKeyUp () {
            GivenPort(3);
            SendAltTab();
            SendEscapeDown();
            SendEscapeUp();

            AssertConsumed();
        }

        [TestMethod]
        public void ConsumedAltF4KeyDownAlsoConsumesItsF4KeyUp () {
            GivenPort(3);
            SendAltTab();
            SendAltF4Down();
            SendAltF4Up();

            AssertConsumed();
        }

    }

}
