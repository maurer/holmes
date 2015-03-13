module Datalog where
open import Data.Nat as ℕ
open import Data.List
open import Data.Integer as ℤ
open import Relation.Binary.PropositionalEquality
open import Data.List.Any

module Term where
  data tt : Set where
    τ-int : tt

  open Membership (setoid tt)

  ctx = List tt

  data Lit : tt → Set where
    int : ℤ → Lit τ-int

  data Expr {τ : tt} {{ Γ : ctx }} {{ Δ : ctx }} : Set where
    lit   : (Lit τ) → Expr {τ}
    e-var : (τ ∈ Γ) → Expr {τ}
    a-var : (τ ∈ Δ) → Expr {τ}

module Program where
  
  at = List Term.tt

  open Membership (setoid at)

  ctx = List at

  data PartialAtom {Γ : Term.ctx} {Δ : Term.ctx} {Ρ : ctx} (ρ : at) : Set where
    predicate : (ρ ∈ Ρ) → PartialAtom ρ
    apply : ∀ {τ : Term.tt} → PartialAtom {Γ} {Δ} {Ρ} (τ ∷ ρ) → Term.Expr {τ} ⦃ Γ ⦄ ⦃ Δ ⦄ → PartialAtom {Γ} {Δ} {Ρ} ρ

  data Atom {Γ : Term.ctx} {Δ : Term.ctx} ⦃ Ρ : ctx ⦄ : Set where
    close : PartialAtom {Γ} {Δ} {Ρ} [] → Atom

  data Rule {Ρ : ctx} : Set where
    -- Here, we should also be checking that the rhs uses Γ to make it a relevant ctx
    -- when used in the universal position
    -- I don't think I need that property for proofs yet, so it can wait
    rule : {Γ : Term.ctx} → Atom {Γ} {[]} → List (Atom {[]} {Γ}) → Rule

  Program : {Ρ : ctx} → Set
  Program {Ρ} = List (Rule {Ρ})
  
  data Derivable {Ρ} : Program {Ρ} → Atom {[]} {[]} → Set where

{-
  Proof sketch:
  0.) match as an assumed-correct-primitive
  1.) Proof of upper bound : prog, ∀ r ∈ prog. ∀ db. ∀ s ∈ match db r.body. subst s r.head ∈ upper
  2.) semantics : p : Program → db → ∀ a ∈ db. Derivable Ρ prog a → upper → (∀ r ∈ p. ∀ vdb. ∀ s ∈ match vdb r.body. let h = subst s r.head in (h ∈ upper) || (h ∈ db))
      a.) Go through prog, trying to generate the proof of termination
      b.) If on any rule you fail, take the failing fact:
          i.) Move the fact from upper to db
          ii.) Add derivability proof for the fact you just derived (look up the matched facts derivation and build up)
          iii.) Fix up the completeness term by demonstrating that if all that happened moved from upper to db, the property is just as true
  3.) Proof of correctness is:
      db, ∀ a ∈ db. Derivable {Ρ} prog a, ∀ r ∈ prog. ∀ s ∈ match db r.body. subst s r.head ∈ db
      e.g. database, proof that database is sound, proof that database is complete
-}
