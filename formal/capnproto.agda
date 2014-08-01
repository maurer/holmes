open import Data.List
open import Data.List.Any
open import Data.Nat as ℕ
data capn-kind : Set where
 cKStruct : capn-kind
 ckInter  : capn-kind
open import Relation.Binary.PropositionalEquality
open Membership (setoid capn-kind)
capn-ctx = List capn-kind
data capn-τ : {Γ : capn-ctx} → Set
data capn-field : {Γ : capn-ctx} → Set
data capn-method : {Γ : capn-ctx} → Set

--While the actual capnp schema supports declaration of structs in any order / arbitrary recursion,
--I am limiting this to reference to structs-declared-so-far. This makes the model simpler, without
--removing any of the power (I think), though it would be less convenient to write code for.

data capn-τ where
  cVoid      : ∀ {Γ} → capn-τ {Γ}
  cBool      : ∀ {Γ} → capn-τ {Γ}
  cInt8      : ∀ {Γ} → capn-τ {Γ} 
  cInt16     : ∀ {Γ} → capn-τ {Γ} 
  cInt32     : ∀ {Γ} → capn-τ {Γ} 
  cInt64     : ∀ {Γ} → capn-τ {Γ} 
  cUInt8     : ∀ {Γ} → capn-τ {Γ} 
  cUInt16    : ∀ {Γ} → capn-τ {Γ} 
  cUInt32    : ∀ {Γ} → capn-τ {Γ} 
  cUInt64    : ∀ {Γ} → capn-τ {Γ} 
  cFloat32   : ∀ {Γ} → capn-τ {Γ} 
  cFloat64   : ∀ {Γ} → capn-τ {Γ} 
  cData      : ∀ {Γ} → capn-τ {Γ} 
  cText      : ∀ {Γ} → capn-τ {Γ}
  cList      : ∀ {Γ} → capn-τ {Γ} → capn-τ {Γ}
  cStruct    : ∀ {Γ} → List (capn-field {cKStruct ∷ Γ}) → capn-τ {cKStruct ∷ Γ}
  cEnum      : ∀ {Γ} → ℕ → capn-τ {Γ}
  cVar       : ∀ {Γ k} → (k ∈ Γ) → capn-τ {Γ}
  cInterface : ∀ {Γ} → List (capn-method {ckInter ∷ Γ}) → capn-τ {ckInter ∷ Γ}

--I'm ignoring field names, because fields can be indexed by list position, which is enough for proofs
--I'm also ignoring groups, since they are just an addressing convenience
data capn-field where
  cField   : ∀ {Γ} → capn-τ {Γ} → capn-field {Γ}
  cUnion   : ∀ {Γ} → List (capn-field {Γ})→ capn-field {Γ}
  cAny     : ∀ {Γ} → capn-field {Γ}

data capn-method where
  cMeth : ∀ {Γ} → List (capn-τ {Γ}) → List (capn-τ {Γ}) → capn-method {Γ}
