/// Creates a static map for an enum with only unit variants
#[macro_export]
macro_rules! mapped_enum {
    (
        $(#[$enum_outer:meta])*
        $enum_vis:vis enum $enum_name:ident {

            $(
                $(#[$enum_inner:meta])*
                $variant:ident $(= $id_val:literal)?
            ),*$(,)?
        }

        $(#[$map_outer:meta])*
        $map_vis:vis struct $map_name:ident;
    ) => {
        $(#[$enum_outer])*
        $enum_vis enum $enum_name {
            $(
                $(#[$enum_inner])*
                $variant $(= $id_val)?
            ),*
        }

        #[allow(non_snake_case)]
        $(#[$map_outer])*
        $map_vis struct $map_name<T> {
            $($variant: T),*
        }

        impl<T> std::ops::Index<$enum_name> for $map_name<T> {
            type Output = T;

            fn index(&self, index: $enum_name) -> &Self::Output {
                match index {
                    $($enum_name::$variant => &self.$variant),*
                }
            }
        }

        impl<T> std::ops::IndexMut<$enum_name> for $map_name<T> {
            fn index_mut(&mut self, index: $enum_name) -> &mut Self::Output {
                match index {
                    $($enum_name::$variant => &mut self.$variant),*
                }
            }
        }

        impl<T: Default> Default for $map_name<T> {
            fn default() -> Self {
                Self {
                    $($variant: Default::default()),*
                }
            }
        }
    };
}
