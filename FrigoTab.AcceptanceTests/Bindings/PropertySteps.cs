using System;
using System.Collections.Generic;
using FrigoTab;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Reqnroll;

namespace FrigoTab.AcceptanceTests.Bindings {

    [Binding]
    public sealed class PropertySteps {

        private Property<string> property;
        private readonly IList<PropertyChange> changes = new List<PropertyChange>();

        [Given(@"a string property with value ""([^"" ]+)""")]
        public void GivenAStringPropertyWithValue (string value) {
            property = new Property<string>();
            property.Value = value;
            property.Changed += OnPropertyChanged;
        }

        [When(@"I assign the property the same value ""([^"" ]+)""")]
        public void WhenIAssignThePropertyTheSameValue (string value) {
            property.Value = value;
        }

        [When(@"I assign the property ""([^"" ]+)""")]
        public void WhenIAssignTheProperty (string value) {
            property.Value = value;
        }

        [Then(@"property change notifications count is (\d+)")]
        public void ThenPropertyChangeNotificationsCountIs (int count) {
            Assert.AreEqual(count, changes.Count);
        }

        [Then(@"the property changed from ""([^"" ]+)"" to ""([^"" ]+)""")]
        public void ThenThePropertyChangedFromTo (string oldValue, string newValue) {
            Assert.AreEqual(1, changes.Count);
            Assert.AreEqual(oldValue, changes[0].OldValue);
            Assert.AreEqual(newValue, changes[0].NewValue);
        }

        private void OnPropertyChanged (string oldValue, string newValue) {
            changes.Add(new PropertyChange(oldValue, newValue));
        }

        private sealed class PropertyChange {

            public PropertyChange (string oldValue, string newValue) {
                OldValue = oldValue;
                NewValue = newValue;
            }

            public string OldValue { get; }
            public string NewValue { get; }

        }

    }

}
