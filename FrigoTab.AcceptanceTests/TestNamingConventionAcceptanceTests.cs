using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Text.RegularExpressions;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("ContractProbe")]
    public sealed class TestNamingConventionAcceptanceTests {

        private static readonly Regex RequiredName = new Regex(
            @"^T\d{8}T\d{6}Z_\d{3}_[A-Z][A-Za-z0-9]*$",
            RegexOptions.CultureInvariant);

        [TestMethod]
        public void T20260912T092700Z_067_AllTestsUseImmutableUtcTimestampPrefixes () {
            MethodInfo[] tests = Assembly.GetExecutingAssembly()
                .GetTypes()
                .SelectMany(type => type.GetMethods(BindingFlags.Public | BindingFlags.Instance))
                .Where(method => method.GetCustomAttribute<TestMethodAttribute>() != null)
                .ToArray();

            string[] invalidNames = tests
                .Select(method => method.Name)
                .Where(name => !RequiredName.IsMatch(name))
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                invalidNames.Length,
                "Every test needs a stable TyyyyMMddTHHmmssZ_NNN_Description prefix. Invalid: " +
                    String.Join(", ", invalidNames));

            string[] duplicateNames = tests
                .GroupBy(method => method.Name, StringComparer.Ordinal)
                .Where(group => group.Count() > 1)
                .Select(group => group.Key)
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                duplicateNames.Length,
                "Timestamp test identifiers must be unique. Duplicates: " +
                    String.Join(", ", duplicateNames));
        }

    }

}
