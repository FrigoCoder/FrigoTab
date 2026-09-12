using System;
using System.Linq;
using System.Reflection;
using FrigoTab.Core;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Reqnroll;

namespace FrigoTab.AcceptanceTests.Bindings {

    [Binding]
    public sealed class ProductionIntegrationSteps {

        private Type sessionFormType;

        [Given(@"the production session form type")]
        public void GivenTheProductionSessionFormType () {
            sessionFormType = typeof(SessionForm);
        }

        [Then(@"it implements the switcher session port")]
        public void ThenItImplementsTheSwitcherSessionPort () {
            RequireType();
            Assert.IsTrue(
                typeof(ISwitcherSessionPort).IsAssignableFrom(sessionFormType),
                "The production session form must adapt native UI operations to the tested switcher policy.");
        }

        [Then(@"it owns a SwitcherApplication controller")]
        public void ThenItOwnsASwitcherApplicationController () {
            RequireType();
            FieldInfo field = sessionFormType.GetFields(BindingFlags.Instance | BindingFlags.NonPublic)
                .SingleOrDefault(candidate => candidate.FieldType == typeof(SwitcherApplication));
            Assert.IsNotNull(field, "The production session form must own the tested SwitcherApplication policy.");
        }

        [Then(@"it exposes the keyboard event entry point")]
        public void ThenItExposesTheKeyboardEventEntryPoint () {
            RequireType();
            MethodInfo method = sessionFormType.GetMethod(
                "HandleKeyEvents",
                BindingFlags.Instance | BindingFlags.Public,
                null,
                new[] {typeof(KeyHookEventArgs)},
                null);
            Assert.IsNotNull(method, "The native hook must be connected to the production session form entry point.");
        }

        private void RequireType () {
            Assert.IsNotNull(sessionFormType, "The production session form type was not selected.");
        }

    }

}
