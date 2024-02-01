use crate::zst::{zst_struct, ZstKind};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{punctuated::Punctuated, token::Comma, Field, Ident, Index, Visibility};

pub fn fields_struct(
    ident: Ident,
    vis: Visibility,
    fields: Punctuated<Field, Comma>,
    kind: FieldKind,
    extra_impl: ExtraImpl,
) -> Result<TokenStream, syn::Error> {
    let fields_len = fields.len();
    let (vis_all, (ty_all, ident_all)): (Vec<_>, (Vec<_>, Vec<FieldIdent>)) = fields
        .into_iter()
        .enumerate()
        .map(|(i, field)| (field.vis, (field.ty, (i, field.ident).into())))
        .unzip();
    let ident_rev: Vec<_> = ident_all.iter().cloned().rev().collect();

    let (_vis_head, ident_head, ty_head) = match (
        vis_all.first().cloned(),
        ty_all.first().cloned(),
        ident_all.first().cloned(),
    ) {
        (Some(vis), Some(ty), Some(ident)) => (vis, ident, ty),
        _ => {
            let zst_kind = match kind {
                FieldKind::Named => ZstKind::Empty,
                FieldKind::Unnamed => ZstKind::EmptyTuple,
            };
            return Ok(zst_struct(ident, vis, zst_kind));
        }
    };

    let _vis_tail: Vec<_> = vis_all.iter().skip(1).cloned().collect();
    let ty_tail: Vec<_> = ty_all.iter().skip(1).cloned().collect();
    let ident_tail: Vec<_> = ident_all.iter().skip(1).cloned().collect();

    let slices = format_ident!("{ident}SoaSlices");
    let slices_mut = format_ident!("{ident}SoaSlicesMut");
    let item_ref = format_ident!("{ident}SoaRef");
    let item_ref_mut = format_ident!("{ident}SoaRefMut");
    let raw = format_ident!("{ident}RawSoa");

    let mut out = TokenStream::new();

    let raw_body = match kind {
        FieldKind::Named => quote! {
            { #(#vis_all #ident_all: ::std::ptr::NonNull<#ty_all>,)* }
        },
        FieldKind::Unnamed => quote! {
            ( #(#vis_all ::std::ptr::NonNull<#ty_all>),* );
        },
    };

    out.append_all(quote! {
        #[automatically_derived]
        #[derive(Copy, Clone)]
        #vis struct #raw #raw_body
    });

    let slices_def = match kind {
        FieldKind::Named => quote! {
            { #(#[automatically_derived] #vis_all #ident_all: &'a [#ty_all]),* }
        },
        FieldKind::Unnamed => quote! {
            ( #(#[automatically_derived] #vis_all &'a [#ty_all]),* );
        },
    };

    out.append_all(quote! {
        #[automatically_derived]
        #[derive(Copy, Clone)]
        #vis struct #slices<'a> #slices_def
    });

    let slices_mut_def = match kind {
        FieldKind::Named => quote! {
            { #(#[automatically_derived] #vis_all #ident_all: &'a mut [#ty_all]),* }
        },
        FieldKind::Unnamed => quote! {
            ( #(#[automatically_derived] #vis_all &'a mut [#ty_all]),* );
        },
    };

    out.append_all(quote! {
        #[automatically_derived]
        #vis struct #slices_mut<'a> #slices_mut_def
    });

    let item_ref_def = match kind {
        FieldKind::Named => quote! {
            { #(#[automatically_derived] #vis_all #ident_all: &'a #ty_all),* }
        },
        FieldKind::Unnamed => quote! {
            ( #(#[automatically_derived] #vis_all &'a #ty_all),* );
        },
    };

    out.append_all(quote! {
        #[automatically_derived]
        #vis struct #item_ref<'a> #item_ref_def
    });

    let item_ref_mut_def = match kind {
        FieldKind::Named => quote! {
            { #(#[automatically_derived] #vis_all #ident_all: &'a mut #ty_all),* }
        },
        FieldKind::Unnamed => quote! {
            ( #(#[automatically_derived] #vis_all &'a mut #ty_all),* );
        },
    };

    out.append_all(quote! {
        #[automatically_derived]
        #vis struct #item_ref_mut<'a> #item_ref_mut_def
    });

    let with_ref_impl = |item| {
        quote! {
            impl<'a> ::soapy_shared::WithRef<#ident> for #item<'a> {
                fn with_ref<F, R>(&self, f: F) -> R
                where
                    F: FnOnce(&#ident) -> R,
                {
                    let t = ::std::mem::ManuallyDrop::new(#ident {
                        #(#ident_all: unsafe { (self.#ident_all as *const #ty_all).read() },)*
                    });
                    f(&t)
                }
            }
        }
    };

    out.append_all(with_ref_impl(item_ref.clone()));
    out.append_all(with_ref_impl(item_ref_mut.clone()));

    if extra_impl.partial_eq {
        // TODO: Impls for item_ref_mut, slices, slices_mut
        out.append_all(quote! {
            impl ::std::cmp::PartialEq for #item_ref {
                fn eq(&self, other: &Self) -> bool {
                    <Self as ::soapy_shared::WithRef<#ident>>::with_ref(self, |me| {
                        <Self as ::soapy_shared::WithRef<#ident>>::with_ref(other, |them| {
                            me == them
                        })
                    })
                }
            }

            impl ::std::cmp::PartialEq<#ident> for #item_ref {
                fn eq(&self, other: &#ident) -> bool {
                    <Self as ::soapy_shared::WithRef<#ident>>::with_ref(self, |me| {
                        me == other
                    })
                }
            }
        })
    }

    let indices = std::iter::repeat(()).enumerate().map(|(i, ())| i);

    out.append_all(quote! {
        #[automatically_derived]
        impl ::soapy_shared::Soapy for #ident {
            type RawSoa = #raw;
            type Slices<'a> = #slices<'a> where Self: 'a;
            type SlicesMut<'a> = #slices_mut<'a> where Self: 'a;
            type Ref<'a> = #item_ref<'a> where Self: 'a;
            type RefMut<'a> = #item_ref_mut<'a> where Self: 'a;
        }

        #[automatically_derived]
        impl #raw {
            #[inline]
            fn layout_and_offsets(cap: usize) -> (::std::alloc::Layout, [usize; #fields_len]) {
                // TODO: Replace unwraps with unwrap_unchecked
                let layout = ::std::alloc::Layout::array::<#ty_head>(cap).unwrap();
                let mut offsets = [0usize; #fields_len];
                let i = 0;
                #(
                    let array = ::std::alloc::Layout::array::<#ty_tail>(cap).unwrap();
                    let (layout, offset) = layout.extend(array).unwrap();
                    offsets[i] = offset;
                    let i = i + 1;
                )*
                (layout, offsets)
            }

            #[inline]
            unsafe fn with_offsets(ptr: *mut u8, offsets: [usize; #fields_len]) -> Self {
                Self {
                    #ident_head: ::std::ptr::NonNull::new_unchecked(ptr as *mut #ty_head),
                    #(
                    #ident_tail: ::std::ptr::NonNull::new_unchecked(
                        ptr.add(offsets[#indices]) as *mut #ty_tail,
                    )
                    ),*
                }
            }
        }

        #[automatically_derived]
        unsafe impl ::soapy_shared::RawSoa<#ident> for #raw {
            #[inline]
            fn dangling() -> Self {
                Self {
                    #(#ident_all: ::std::ptr::NonNull::dangling(),)*
                }
            }

            #[inline]
            unsafe fn slices(&self, start: usize, end: usize) -> #slices<'_> {
                let len = end - start;
                #slices {
                    #(
                    #ident_all: ::std::slice::from_raw_parts(
                        self.#ident_all.as_ptr().add(start),
                        len,
                    ),
                    )*
                }
            }

            #[inline]
            unsafe fn slices_mut(&mut self, start: usize, end: usize) -> #slices_mut<'_> {
                let len = end - start;
                #slices_mut {
                    #(
                    #ident_all: ::std::slice::from_raw_parts_mut(
                        self.#ident_all.as_ptr().add(start),
                        len,
                    ),
                    )*
                }
            }

            #[inline]
            fn as_ptr(self) -> *mut u8 {
                self.#ident_head.as_ptr() as *mut u8
            }

            #[inline]
            unsafe fn from_parts(ptr: *mut u8, capacity: usize) -> Self {
                let (_, offsets) = Self::layout_and_offsets(capacity);
                Self::with_offsets(ptr, offsets)
            }

            #[inline]
            unsafe fn alloc(capacity: usize) -> Self {
                let (new_layout, new_offsets) = Self::layout_and_offsets(capacity);
                let ptr = ::std::alloc::alloc(new_layout);
                assert_ne!(ptr as *const u8, ::std::ptr::null());
                Self::with_offsets(ptr, new_offsets)
            }

            #[inline]
            unsafe fn realloc_grow(&mut self, old_capacity: usize, new_capacity: usize, length: usize) {
                let (new_layout, new_offsets) = Self::layout_and_offsets(new_capacity);
                let (old_layout, old_offsets) = Self::layout_and_offsets(old_capacity);
                // Grow allocation first
                let ptr = self.#ident_head.as_ptr() as *mut u8;
                let ptr = ::std::alloc::realloc(ptr, old_layout, new_layout.size());
                assert_ne!(ptr as *const u8, ::std::ptr::null());
                // Pointer may have moved, can't reuse self
                let old = Self::with_offsets(ptr, old_offsets);
                let new = Self::with_offsets(ptr, new_offsets);
                // Copy do destination in reverse order to avoid
                // overwriting data
                #(::std::ptr::copy(old.#ident_rev.as_ptr(), new.#ident_rev.as_ptr(), length);)*
                *self = new;
            }

            #[inline]
            unsafe fn realloc_shrink(&mut self, old_capacity: usize, new_capacity: usize, length: usize) {
                let (old_layout, _) = Self::layout_and_offsets(old_capacity);
                let (new_layout, new_offsets) = Self::layout_and_offsets(new_capacity);
                // Move data before reallocating as some data
                // may be past the end of the new allocation.
                // Copy from front to back to avoid overwriting data.
                let ptr = self.#ident_head.as_ptr() as *mut u8;
                let dst = Self::with_offsets(ptr, new_offsets);
                #(::std::ptr::copy(self.#ident_all.as_ptr(), dst.#ident_all.as_ptr(), length);)*
                let ptr = ::std::alloc::realloc(ptr, old_layout, new_layout.size());
                assert_ne!(ptr as *const u8, ::std::ptr::null());
                // Pointer may have moved, can't reuse dst
                *self = Self::with_offsets(ptr, new_offsets);
            }

            #[inline]
            unsafe fn dealloc(self, old_capacity: usize) {
                let (layout, _) = Self::layout_and_offsets(old_capacity);
                ::std::alloc::dealloc(self.as_ptr(), layout);
            }

            #[inline]
            unsafe fn copy(&mut self, src: usize, dst: usize, count: usize) {
                #(
                    let ptr = self.#ident_all.as_ptr();
                    ::std::ptr::copy(ptr.add(src), ptr.add(dst), count);
                )*
            }

            #[inline]
            unsafe fn set(&mut self, index: usize, element: #ident) {
                #(self.#ident_all.as_ptr().add(index).write(element.#ident_all);)*
            }

            #[inline]
            unsafe fn get(&self, index: usize) -> #ident {
                #ident {
                    #(#ident_all: self.#ident_all.as_ptr().add(index).read(),)*
                }
            }

            #[inline]
            unsafe fn get_ref<'a>(&self, index: usize) -> #item_ref<'a> {
                #item_ref {
                    #(#ident_all: self.#ident_all.as_ptr().add(index).as_ref().unwrap_unchecked(),)*
                }
            }

            #[inline]
            unsafe fn get_mut<'a>(&self, index: usize) -> #item_ref_mut<'a> {
                #item_ref_mut {
                    #(#ident_all: self.#ident_all.as_ptr().add(index).as_mut().unwrap_unchecked(),)*
                }
            }
        }
    });

    Ok(out)
}

#[derive(Clone, PartialEq, Eq)]
enum FieldIdent {
    Named(Ident),
    Unnamed(Index),
}

impl From<(usize, Option<Ident>)> for FieldIdent {
    fn from(value: (usize, Option<Ident>)) -> Self {
        match value {
            (_, Some(ident)) => Self::Named(ident),
            (i, None) => Self::Unnamed(Index::from(i)),
        }
    }
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldIdent::Named(ident) => ident.to_tokens(tokens),
            FieldIdent::Unnamed(i) => i.to_tokens(tokens),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldKind {
    Named,
    Unnamed,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct ExtraImpl {
    pub debug: bool,
    pub partial_eq: bool,
}
