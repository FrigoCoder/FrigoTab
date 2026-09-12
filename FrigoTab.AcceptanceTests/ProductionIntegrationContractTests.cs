using System;
using System.Linq;
using System.Reflection;
using FrigoTab;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("Regression")]
    [TestCategory("ContractProbe")]
    public sealed class ProductionIntegrationContractTests {

        private static readonly Type SessionFormType = typeof(SessionForm);

        [TestMethod]
        public void T20260912T090300Z_037_SessionFormIsWiredToTheTestedSwitcherPolicy () {
            Assert.IsTrue(
                typeof(ISwitcherSessionPort).IsAssignableFrom(SessionFormType),
                "The production session form must adapt native UI operations to the tested switcher policy.");

            FieldInfo controller = SessionFormType.GetFields(BindingFlags.Instance | BindingFlags.NonPublic)
                .SingleOrDefault(candidate => candidate.FieldType == typeof(SwitcherApplication));
            Assert.IsNotNull(controller, "The production session form must own the tested SwitcherApplication policy.");

            MethodInfo keyboardEntryPoint = SessionFormType.GetMethod(
                "HandleKeyEvents",
                BindingFlags.Instance | BindingFlags.Public,
                null,
                new[] {typeof(KeyHookEventArgs)},
                null);
            Assert.IsNotNull(keyboardEntryPoint, "The native hook must be connected to the production session form entry point.");
        }

    }

}
