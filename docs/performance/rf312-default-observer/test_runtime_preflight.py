import datetime as dt
import unittest
from runtime_preflight import fresh_anr_rows


class CompleteFreshAnr(unittest.TestCase):
    def setUp(self):
        self.birth=dt.datetime.fromisoformat('2027-01-01T00:00:00+00:00')
        self.rows=['2027-01-01 00:00:01 SYSTEM_BOOT (text, 8 bytes)',
                   '2027-01-01 00:00:02 system_server_strictmode (text, 12 bytes)']
        self.last='<no ANR has occurred since boot>'

    def text(self,rows=None,count=None):
        rows=self.rows if rows is None else rows
        return f'Drop box contents: {len(rows) if count is None else count} entries\nMax entries: 1000\n'+'\n'.join(rows)+'\n'

    def test_complete_uncapped_new_boot_is_not_tied_to_a_calendar_year(self):
        self.assertEqual(len(fresh_anr_rows(self.text(),self.last,self.birth)),2)

    def test_truncated_empty_or_count_mismatched_inventory_refuses(self):
        for value in (self.text(count=3),self.text(rows=[]),self.text(rows=self.rows[:1],count=2)):
            with self.subTest(value=value),self.assertRaises(AssertionError):fresh_anr_rows(value,self.last,self.birth)

    def test_cap_hit_cannot_hide_old_anr_entries(self):
        rows=[self.rows[0]]*1000
        with self.assertRaises(AssertionError):fresh_anr_rows(self.text(rows=rows),self.last,self.birth)

    def test_prebirth_history_and_unsorted_rows_refuse_without_filtering(self):
        for rows in ([self.rows[0].replace('2027','2026')]+self.rows,list(reversed(self.rows))):
            with self.subTest(rows=rows),self.assertRaises(AssertionError):fresh_anr_rows(self.text(rows=rows),self.last,self.birth)

    def test_every_anr_tag_is_rejected_regardless_of_age_or_target(self):
        for tag in ('system_app_anr','data_app_anr','system_server_anr'):
            rows=self.rows+['2027-01-01 00:00:03 '+tag+' (text, 20 bytes)']
            with self.subTest(tag=tag),self.assertRaises(AssertionError):fresh_anr_rows(self.text(rows=rows),self.last,self.birth)

    def test_last_anr_and_missing_limit_proof_cannot_pass(self):
        with self.assertRaises(AssertionError):fresh_anr_rows(self.text(),'actual ANR evidence',self.birth)
        with self.assertRaises(AssertionError):fresh_anr_rows(self.text().replace('Max entries: 1000','Max entries: 10'),self.last,self.birth)


if __name__=='__main__':unittest.main()
