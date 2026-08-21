(function () {
  var config = { signatureTimestamp: 20676 };

  function Cipher(value) {
    this.values = { s: value, n: null };
  }
  Cipher.prototype.set = function (key, value) {
    this.values[key] = value;
  };
  Cipher.prototype.get = function (key) {
    return this.values[key];
  };
  Cipher.prototype.clone = function () {
    return this;
  };
  Cipher.prototype.transform = function () {
    if (this.values.s) this.values.s = this.values.s.split('').reverse().join('');
    if (this.values.n) this.values.n = `n-${this.values.n}`;
  };
  Cipher.prototype.marker = function () {};

  function solve(url, key, signature) {
    var result = new Cipher(signature);
    result.marker('alr', 'yes');
    return result;
  }
}).call(this);
