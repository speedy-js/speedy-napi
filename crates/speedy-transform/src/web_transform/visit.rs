use swc_ecma_ast::{Ident, ImportDecl, JSXElement, JSXElementName, JSXObject};
use swc_ecma_visit::noop_visit_type;
use swc_ecma_visit::Visit;
use swc_ecma_visit::VisitWith;

#[derive(Default)]
pub struct IdentComponent {
  pub component_name_jsx_ident: Vec<(String, u32)>,
  pub ident_list: Vec<(String, u32)>,
}

///
/// 处理 babel_import 自动 treeshaking 的问题
/// 增加 判断 jsx 所有引用的关系
///
impl Visit for IdentComponent {
  // need to skip import decl
  fn visit_import_decl(&mut self, _: &ImportDecl) {}

  fn visit_jsx_element(&mut self, jsx: &JSXElement) {
    let mut compent_name = match &jsx.opening.name {
      JSXElementName::Ident(ident) => (ident.to_string(), ident.span.ctxt.as_u32()),
      JSXElementName::JSXMemberExpr(member) => {
        let mut obj = &member.obj;
        let real_ident;
        loop {
          match obj {
            JSXObject::JSXMemberExpr(next) => obj = &next.obj,
            JSXObject::Ident(ident) => {
              real_ident = ident;
              break;
            }
          }
        }

        (real_ident.to_string(), real_ident.span.ctxt.as_u32())
      }
      JSXElementName::JSXNamespacedName(space) => {
        (space.name.to_string(), space.name.span.ctxt.as_u32())
      }
    };
    compent_name.0 = compent_name.0.replace("#0", "");
    self.component_name_jsx_ident.push(compent_name);
    jsx.children.visit_with(self);
  }

  fn visit_ident(&mut self, ident: &Ident) {
    self
      .ident_list
      .push((ident.sym.to_string(), ident.span.ctxt.as_u32()));
  }

  noop_visit_type!();
}
