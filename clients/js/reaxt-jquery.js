/*! reaxt-jquery: jQuery プラグイン。jQuery 1.x〜4.x(Deferred と $.ajax があれば動く)。
 *  $.reaxt.call("demo.v1.Counter", "Inc", null).done(function (r) { ... }).fail(function (e) { ... });
 *  $("#btn").reaxt("click", "demo.v1.Counter", "Inc", function () { return null; }, function (r) { $("#n").text(r); });
 */
(function (root, factory) {
  if (typeof module === "object" && module.exports) module.exports = factory(require("jquery"));
  else factory(root.jQuery);
})(typeof self !== "undefined" ? self : this, function ($) {
  "use strict";
  var base = "";
  $.reaxt = {
    /** 接続先とヘッダーを設定する */
    config: function (opts) { base = (opts && opts.baseUrl || "").replace(/\/$/, ""); this.headers = opts && opts.headers || {}; return this; },
    headers: {},
    /** Connect 単純呼び出し。jqXHR 互換の Promise を返す。失敗時は {status, code, message}。 */
    call: function (service, method, input) {
      var d = $.Deferred();
      $.ajax({
        url: base + "/" + service + "/" + method,
        method: "POST",
        contentType: "application/json",
        dataType: "json",
        data: JSON.stringify(input === undefined ? null : input),
        headers: $.extend({ "x-reaxt": "1" }, $.reaxt.headers)
      }).done(function (r) { d.resolve(r); })
        .fail(function (x) {
          var j = x.responseJSON || {};
          d.reject({ status: x.status, code: j.code || "unknown", message: j.message || x.responseText || "" });
        });
      return d.promise();
    },
    manifest: function () { return $.getJSON(base + "/_reaxt/manifest.json").then(function (m) { return m.actions; }); }
  };
  /** 要素のイベントでアクションを呼ぶ。入力は関数(this=要素)、結果はコールバックへ。 */
  $.fn.reaxt = function (event, service, method, input, done, fail) {
    return this.on(event, function (e) {
      e.preventDefault();
      var arg = typeof input === "function" ? input.call(this, e) : input;
      $.reaxt.call(service, method, arg).done(done && done.bind(this)).fail(fail && fail.bind(this));
    });
  };
  return $;
});
