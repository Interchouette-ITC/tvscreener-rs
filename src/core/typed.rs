// Copyright 2026 tvscreener-rs contributors
// SPDX-License-Identifier: Apache-2.0

//! Shared boilerplate for typed asset screeners.

/// Implements common typed-screener methods that forward to [`super::Screener`].
macro_rules! impl_typed_screener_common {
    ($name:ident) => {
        impl $name {
            /// Immutable access to the shared builder.
            #[must_use]
            pub const fn inner(&self) -> &$crate::core::Screener {
                &self.inner
            }

            /// Mutable access for advanced configuration.
            pub const fn inner_mut(&mut self) -> &mut $crate::core::Screener {
                &mut self.inner
            }

            /// Runs the scan
            ///
            /// # Errors
            ///
            /// Propagates errors from [`$crate::core::Screener::get`].
            pub async fn get(&self) -> $crate::error::Result<Vec<$crate::ScreenerRow>> {
                self.inner.get().await
            }

            /// Replaces selected columns
            pub fn select(
                &mut self,
                fields: impl IntoIterator<Item = $crate::field::FieldDef>,
            ) -> &mut Self {
                self.inner.select(fields);
                self
            }

            /// Reloads default columns via [`$crate::core::Screener::select_all`].
            ///
            /// # Panics
            ///
            /// Panics if `set_all_fields_fn` was not registered in `new()` (typed constructors always do).
            pub fn select_all(&mut self) -> &mut Self {
                self.inner.select_all().expect(concat!(
                    stringify!($name),
                    "::new always sets set_all_fields_fn"
                ));
                self
            }

            /// Adds a name/description search filter.
            ///
            /// # Errors
            ///
            /// Propagates [`$crate::core::Screener::search`] errors.
            pub fn search(&mut self, value: impl Into<String>) -> $crate::error::Result<&mut Self> {
                self.inner.search(value)?;
                Ok(self)
            }

            /// Sets the result window
            pub const fn set_range(&mut self, from_range: u32, to_range: u32) -> &mut Self {
                self.inner.set_range(from_range, to_range);
                self
            }

            /// Adds a filter from a [`$crate::filter::FieldCondition`].
            ///
            /// # Errors
            ///
            /// Propagates [`$crate::core::Screener::where_condition`] errors.
            pub fn where_condition(
                &mut self,
                condition: $crate::filter::FieldCondition,
            ) -> $crate::error::Result<&mut Self> {
                self.inner.where_condition(condition)?;
                Ok(self)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}
