/* eslint-disable */
/* prettier-ignore */
// @ts-nocheck
// Generated by unplugin-vue-router. ‼️ DO NOT MODIFY THIS FILE ‼️
// It's recommended to commit this file.
// Make sure to add this file to your tsconfig.json file as an "includes" or "files" entry.

declare module 'vue-router/auto-routes' {
  import type {
    RouteRecordInfo,
    ParamValue,
    ParamValueOneOrMore,
    ParamValueZeroOrMore,
    ParamValueZeroOrOne,
  } from 'vue-router'

  /**
   * Route name map generated by unplugin-vue-router
   */
  export interface RouteNamedMap {
    '/': RouteRecordInfo<'/', '/', Record<never, never>, Record<never, never>>,
    '/catalogue/': RouteRecordInfo<'/catalogue/', '/catalogue', Record<never, never>, Record<never, never>>,
    '/catalogue/[id]': RouteRecordInfo<'/catalogue/[id]', '/catalogue/:id', { id: ParamValue<true> }, { id: ParamValue<false> }>,
    '/meta_import': RouteRecordInfo<'/meta_import', '/meta_import', Record<never, never>, Record<never, never>>,
  }
}
