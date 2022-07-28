use std::collections::HashSet;

use swc::Compiler;
use swc_common::{util::take::Take, Mark};
use swc_ecma_ast::{BlockStmt, Expr, Id, Module, ModuleDecl, ModuleExportName, ModuleItem};
use swc_ecma_transforms::resolver;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::types::TransformConfig;
use crate::web_transform::clear_mark::ClearMark;

struct RmUseEffect {
  use_effect_mark: Option<Id>, // used for remove useEffect()
  react_mark: Option<Id>,      // used for remove React.useEffect()
}

const USE_EFFECT_STR: &str = "useEffect";

impl VisitMut for RmUseEffect {
  fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
    let mut rm_idx_set = HashSet::new();
    for (idx, stmt) in n.stmts.iter().enumerate() {
      if let Some(Some(call_expr)) = stmt.as_expr().map(|expr_stmt| expr_stmt.expr.as_call()) {
        if let Some(callee) = call_expr.callee.as_expr() {
          match &**callee {
            Expr::Member(member) => {
              // check React.useEffect call
              if let Some(obj_ident) = member.obj.as_ident() {
                if self.react_mark.is_some() && self.react_mark == Some(obj_ident.to_id()) {
                  if let Some(method_ident) = member.prop.as_ident() {
                    if &method_ident.sym == USE_EFFECT_STR {
                      rm_idx_set.insert(idx);
                    }
                  }
                }
              }
            }
            Expr::Ident(ident) => {
              // check useEffect call
              if self.use_effect_mark.eq(&Some(ident.to_id())) {
                rm_idx_set.insert(idx);
              }
            }
            _ => {}
          };
        }
      }
    }
    if !rm_idx_set.is_empty() {
      n.stmts = n
        .stmts
        .take()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, stmt)| {
          if rm_idx_set.contains(&idx) {
            None
          } else {
            Some(stmt)
          }
        })
        .collect();
    }
    n.stmts.visit_mut_with(self);
  }
}

pub fn remove_call(module: &mut Module, config: &TransformConfig, compiler: &Compiler) {
  if config.remove_use_effect.is_none() || !config.remove_use_effect.unwrap() {
    return;
  }

  compiler.run(|| {
    module.visit_mut_with(&mut resolver(Mark::new(), Mark::new(), false));
  });

  let mut visitor = RmUseEffect {
    react_mark: None,
    use_effect_mark: None,
  };
  for item in &module.body {
    if let ModuleItem::ModuleDecl(ModuleDecl::Import(var)) = item {
      let source = &*var.src.value;
      if source == "react" {
        for specifier in &var.specifiers {
          match specifier {
            swc_ecma_ast::ImportSpecifier::Named(named) => {
              match &named.imported {
                Some(imported) => {
                  let imported_name = match imported {
                    ModuleExportName::Ident(ident) => &ident.sym,
                    ModuleExportName::Str(str) => &str.value,
                  };
                  if imported_name.as_ref() == USE_EFFECT_STR {
                    // import { useEffect as ??? } from 'react'
                    visitor.use_effect_mark = Some(named.local.to_id())
                  }
                }
                None => {
                  if named.local.sym.as_ref() == USE_EFFECT_STR {
                    // import { useEffect } from 'react'
                    visitor.use_effect_mark = Some(named.local.to_id())
                  }
                }
              }
            }
            swc_ecma_ast::ImportSpecifier::Default(default) => {
              // import ??? from 'react'
              visitor.react_mark = Some(default.local.to_id());
            }
            _ => {}
          }
        }
      }
    }
  }

  module.visit_mut_with(&mut visitor);
  module.visit_mut_with(&mut ClearMark);
}
