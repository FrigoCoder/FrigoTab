using System;
using System.Threading;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    public sealed class SingleInstanceAcceptanceTests {

        [TestMethod]
        public void T20260912T091800Z_065_CompetingThreadCannotAcquireApplicationMutex () {
            string name = "Local\\FrigoTab.AcceptanceTests." + Guid.NewGuid().ToString("N");
            SingleInstanceGuard first;
            Assert.IsTrue(SingleInstanceGuard.TryAcquire(name, out first));

            bool secondAcquired = true;
            Exception failure = null;
            Thread contender = new Thread(() => {
                try {
                    SingleInstanceGuard second;
                    secondAcquired = SingleInstanceGuard.TryAcquire(name, out second);
                    second?.Dispose();
                }
                catch( Exception exception ) {
                    failure = exception;
                }
            });

            try {
                contender.Start();
                Assert.IsTrue(contender.Join(TimeSpan.FromSeconds(5)), "The non-blocking acquisition should finish immediately.");
                Assert.IsNull(failure);
                Assert.IsFalse(secondAcquired);
            }
            finally {
                first.Dispose();
            }
        }

    }

}
